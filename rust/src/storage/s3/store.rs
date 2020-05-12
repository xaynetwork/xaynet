use crate::{
    mask::{Integers, Mask, MaskedModel},
    model::Model,
};
use bincode;
use futures::ready;
use rusoto_core::{Region, RusotoError};
use rusoto_s3::{
    CreateBucketError,
    CreateBucketOutput,
    CreateBucketRequest,
    Delete,
    DeleteObjectsError,
    DeleteObjectsOutput,
    DeleteObjectsRequest,
    GetObjectError,
    GetObjectOutput,
    GetObjectRequest,
    ListObjectsV2Error,
    ListObjectsV2Output,
    ListObjectsV2Request,
    ObjectIdentifier,
    PutObjectError,
    PutObjectOutput,
    PutObjectRequest,
    S3Client,
    StreamingBody,
    S3,
};
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;
use tokio::{io::AsyncReadExt, stream::Stream};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("S3Error")]
    S3Error,
}

#[derive(Clone)]
struct Buckets(HashMap<&'static str, &'static str>);

impl Buckets {
    fn new() -> Self {
        let mut names = HashMap::new();
        names.insert("masks", "masks");
        names.insert("masked-models", "masked-models");
        names.insert("global-model", "global-model");
        names.insert("global-masked-model", "global-masked-model");
        Self(names)
    }

    fn masks(&self) -> &'static str {
        self.0.get("masks").unwrap()
    }

    fn masked_models(&self) -> &'static str {
        self.0.get("masked-models").unwrap()
    }

    fn global_model(&self) -> &'static str {
        self.0.get("global-model").unwrap()
    }

    fn global_masked_model(&self) -> &'static str {
        self.0.get("global-masked-model").unwrap()
    }

    fn names(&self) -> impl Iterator<Item = &&str> {
        self.0.values().into_iter()
    }
}

#[derive(Clone)]
pub struct S3Store {
    s3_client: S3Client,
    buckets: Buckets,
}

// API
impl S3Store {
    /// Create a new S3 store. The store creates and maintains 4 different buckets.
    /// See [`Buckets`] for more information.
    ///
    /// The [`S3Store`] can also be used together with Minio via a custom region:
    /// ```text
    /// let region = Region::Custom {
    ///    name: String::from("eu-east-3"),
    ///    endpoint: String::from("http://127.0.0.1:9000"), // URL of minio
    /// };
    ///
    /// let store = S3Store::new(region);
    /// ```
    pub fn new(region: Region) -> Self {
        Self {
            s3_client: S3Client::new(region),
            buckets: Buckets::new(),
        }
    }

    /// Upload a [`Mask`].
    pub async fn upload_mask(&self, key: &str, mask: &Mask) -> Result<(), StorageError> {
        match self
            .upload(self.buckets.masks(), key, mask.serialize())
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(StorageError::S3Error),
        }
    }

    /// Upload a [`MaskedModel`].
    pub async fn upload_masked_model(
        &self,
        key: &str,
        masked_model: &MaskedModel,
    ) -> Result<(), StorageError> {
        // we don't need the keys of the masked models. we can retrieve all keys via `list objects`.
        // maybe we need the number of objects in the bucket.
        // the key is a random string, we could calculate the hash of a masked model and use this
        // as a key but this would cost more time to calculate
        match self
            .upload(self.buckets.masked_models(), &key, masked_model.serialize())
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(StorageError::S3Error),
        }
    }

    /// Upload a global [`Model`].
    pub async fn upload_global_model<N>(
        &self,
        key: &str,
        global_model: &Model<N>,
    ) -> Result<(), StorageError>
    where
        N: serde::Serialize,
    {
        // As key for the global model we use the seed of the round in which the global model was
        // crated. we might also store the round number for easier debugging.
        // the seed is stored in redis so we can recover this key in case of a failure.
        // the seed needs to be returned in the round parameters.
        // {
        //  round_seed : new_seed
        //  global_model: seed_from_the_round_before
        //}
        // we will need to store the global_model_key in redis.
        let se_global_model =
            bincode::serialize(global_model).map_err(|_| StorageError::S3Error)?;

        match self
            .upload(self.buckets.global_model(), key, se_global_model)
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(StorageError::S3Error),
        }
    }

    /// Upload a global [`MaskedModel`].
    pub async fn upload_global_masked_model(
        &self,
        key: &str,
        global_masked_model: &MaskedModel,
    ) -> Result<(), StorageError> {
        // Same like `upload_global_model`. we want to store the global_masked_model because if the
        // aggregator fails between the aggregation and unmasking part, the aggregator will have to
        // do the work again.
        match self
            .upload(
                self.buckets.global_masked_model(),
                key,
                global_masked_model.serialize(),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(StorageError::S3Error),
        }
    }

    /// Download a [`Mask`].
    pub async fn download_mask(&self, key: &str) -> Result<Option<Mask>, StorageError> {
        // mask_hash is stored in the mask_dict
        let object_resp = self
            .download_object(self.buckets.masks(), &key)
            .await
            .map_err(|_| StorageError::S3Error)?;

        if let Some(de_mask) = S3Store::unpack_object(object_resp).await {
            Mask::deserialize(&de_mask)
                .map_err(|_| StorageError::S3Error)
                .and_then(|mask| Ok(Some(mask)))
        } else {
            Ok(None)
        }
    }

    /// Download a [`MaskedModel`].
    pub async fn download_masked_model_id(
        &self,
        id: ObjectIdentifier,
    ) -> Result<Option<MaskedModel>, StorageError> {
        let object_resp = self
            .download_object(self.buckets.masked_models(), &id.key)
            .await
            .map_err(|_| StorageError::S3Error)?;

        if let Some(de_masked_model) = S3Store::unpack_object(object_resp).await {
            MaskedModel::deserialize(&de_masked_model)
                .map_err(|_| StorageError::S3Error)
                .and_then(|mask_model| Ok(Some(mask_model)))
        } else {
            Ok(None)
        }
    }

    /// Download a global [`Model`].
    pub async fn download_global_model<N>(
        &self,
        key: &str,
    ) -> Result<Option<Model<N>>, StorageError>
    where
        N: for<'de> serde::Deserialize<'de>,
    {
        let object_resp = self
            .download_object(self.buckets.global_model(), key)
            .await
            .map_err(|_| StorageError::S3Error)?;

        if let Some(de_global_model) = S3Store::unpack_object(object_resp).await {
            bincode::deserialize(&de_global_model)
                .map_err(|_| StorageError::S3Error)
                .and_then(|global_model| Ok(Some(global_model)))
        } else {
            Ok(None)
        }
    }

    /// Download a global masked model.
    pub async fn download_masked_global_model(
        &self,
        key: &str,
    ) -> Result<Option<MaskedModel>, StorageError> {
        let object_resp = self
            .download_object(self.buckets.global_masked_model(), key)
            .await
            .map_err(|_| StorageError::S3Error)?;

        if let Some(de_global_masked_model) = S3Store::unpack_object(object_resp).await {
            MaskedModel::deserialize(&de_global_masked_model)
                .map_err(|_| StorageError::S3Error)
                .and_then(|masked_global_model| Ok(Some(masked_global_model)))
        } else {
            Ok(None)
        }
    }

    /// Return a stream that yields all objects keys in the masked model bucket.
    pub fn get_masked_model_identifier_stream(&self) -> ListObjectsStream {
        ListObjectsStream::new(self.s3_client.clone(), self.buckets.masked_models(), 10)
    }

    /// Delete all objects in all [`Buckets`].
    pub async fn clear_all(&self) -> Result<(), StorageError> {
        for bucket in self.buckets.names() {
            let _ = self
                .clear_bucket(bucket)
                .await
                .map_err(|_| StorageError::S3Error)?;
        }
        Ok(())
    }

    /// Create all [`Buckets`].
    pub async fn create_buckets(&self) -> Result<(), StorageError> {
        for bucket in self.buckets.names() {
            let resp = self.create_bucket(bucket).await;

            if let Err(RusotoError::Service(CreateBucketError::BucketAlreadyExists(_)))
            | Err(RusotoError::Service(CreateBucketError::BucketAlreadyOwnedByYou(_))) = resp
            {
                continue;
            } else {
                return Err(StorageError::S3Error);
            }
        }
        Ok(())
    }
}

// private methods
impl S3Store {
    /// Upload an object to the given bucket.
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
    ) -> Result<PutObjectOutput, RusotoError<PutObjectError>> {
        let req = PutObjectRequest {
            bucket: bucket.to_string(),
            key: key.to_string(),
            body: Some(StreamingBody::from(data)),
            ..Default::default()
        };

        self.s3_client.put_object(req).await
    }

    /// Download an object from the given bucket.
    async fn download_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<GetObjectOutput, RusotoError<GetObjectError>> {
        let req = GetObjectRequest {
            bucket: bucket.to_string(),
            key: key.to_string(),
            ..Default::default()
        };

        self.s3_client.get_object(req).await
    }

    // Get the content of the given object.
    async fn unpack_object(object: GetObjectOutput) -> Option<Vec<u8>> {
        if let Some(body) = object.body {
            let mut content = Vec::new();
            // TODO handle error
            let _ = body
                .into_async_read()
                .read_to_end(&mut content)
                .await
                .map_err(|_| StorageError::S3Error)
                .ok()?;
            Some(content)
        } else {
            None
        }
    }

    // Create a new bucket with the given bucket name.
    async fn create_bucket(
        &self,
        bucket: &str,
    ) -> Result<CreateBucketOutput, RusotoError<CreateBucketError>> {
        let req = CreateBucketRequest {
            bucket: bucket.to_string(),
            ..Default::default()
        };
        self.s3_client.create_bucket(req).await
    }

    // Delete all objects in a bucket.
    async fn clear_bucket(&self, bucket: &str) -> Result<(), StorageError> {
        let mut continuation_token: Option<String> = None;

        loop {
            let list_obj_resp = self
                .list_objects(bucket, continuation_token)
                .await
                .map_err(|_| StorageError::S3Error)?;

            if let Some(identifiers) = S3Store::unpack_object_identifier(&list_obj_resp) {
                self.delete_objects(bucket, identifiers)
                    .await
                    .map_err(|_| StorageError::S3Error)?;
            } else {
                break;
            }

            // check if more objects exists
            continuation_token = S3Store::unpack_next_continuation_token(&list_obj_resp);
            if continuation_token.is_none() {
                break;
            }
        }
        Ok(())
    }

    // Get the object identifier/keys.
    fn unpack_object_identifier(
        list_obj_resp: &ListObjectsV2Output,
    ) -> Option<Vec<ObjectIdentifier>> {
        if let Some(objects) = &list_obj_resp.contents {
            let keys = objects
                .into_iter()
                .filter_map(|obj| obj.key.clone())
                .map(|key| ObjectIdentifier {
                    key,
                    ..Default::default()
                })
                .collect();
            Some(keys)
        } else {
            None
        }
    }

    // Delete the objects of the given bucket.
    async fn delete_objects(
        &self,
        bucket: &str,
        identifiers: Vec<ObjectIdentifier>,
    ) -> Result<DeleteObjectsOutput, RusotoError<DeleteObjectsError>> {
        let req = DeleteObjectsRequest {
            bucket: bucket.to_string(),
            delete: Delete {
                objects: identifiers,
                ..Default::default()
            },
            ..Default::default()
        };

        self.s3_client.delete_objects(req).await
    }

    // Return all objects keys for the given bucket.
    async fn list_objects(
        // the response returns 1000 keys max.
        // https://docs.aws.amazon.com/AWSJavaScriptSDK/latest/AWS/S3.html#listObjectsV2-property
        &self,
        bucket: &str,
        continuation_token: Option<String>,
    ) -> Result<ListObjectsV2Output, RusotoError<ListObjectsV2Error>> {
        // list all objects
        let req = ListObjectsV2Request {
            bucket: bucket.to_string(),
            continuation_token,
            // Minio is not limited to 1000
            max_keys: Some(1000),
            ..Default::default()
        };

        self.s3_client.list_objects_v2(req).await
    }

    // Unpack the next_continuation_token of the ListObjectsV2Output response.
    fn unpack_next_continuation_token(list_obj_resp: &ListObjectsV2Output) -> Option<String> {
        // https://docs.aws.amazon.com/AmazonS3/latest/dev/ListingObjectKeysUsingJava.html
        if let Some(is_truncated) = list_obj_resp.is_truncated {
            if is_truncated {
                list_obj_resp.next_continuation_token.clone()
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// AWS paginated results when the response object is too large to return in a single response.
/// The maximum number of keys that can be returned in a single request is limited to 1000.
/// If a bucket contains more then 1000 objects, the [`ListObjectsStream`] can be used to easily
/// iterate through the all the object identifiers/keys of a bucket.
pub struct ListObjectsStream {
    /// An instance of a S3Client
    s3_client: S3Client,
    /// The name of the S3 bucket
    bucket: String,
    /// The maximum number of keys returned in a single iteration.
    max_keys: i64,
    /// A Future that resolves when S3 responded to a list_objects request.
    list_object_ids_future: Option<ListObjectIdentifierFuture>,
}

impl ListObjectsStream {
    /// Create a new [`ListObjectsStream`] for the given bucket.
    fn new<S>(s3_client: S3Client, bucket: S, max_keys: i64) -> Self
    where
        S: Into<String>,
    {
        let bucket = bucket.into();
        Self {
            s3_client: s3_client.clone(),
            bucket: bucket.clone(),
            max_keys: max_keys,
            list_object_ids_future: Some(ListObjectIdentifierFuture::new(
                s3_client, bucket, None, max_keys,
            )),
        }
    }

    /// Try to resolve the current [`ListObjectIdentifierFuture`].
    /// If the future is ready, the [`poll_object_identifiers`] function will yield the received
    /// object identifiers. If a single S3 response does not contain all object identifiers of a
    /// bucket, the function will continue to create new [`ListObjectIdentifierFuture`]s until all
    /// object identifiers have been received.
    fn poll_object_identifiers(&mut self, cx: &mut Context) -> Poll<Option<Vec<ObjectIdentifier>>> {
        let fut = if let Some(ref mut fut) = self.list_object_ids_future {
            fut
        } else {
            // if no future exist, all object identifiers have been received
            return Poll::Ready(None);
        };

        match ready!(Pin::new(fut).poll(cx)) {
            Ok(ListObjectIdentifierResult {
                object_identifiers,
                next_continuation_token,
            }) => {
                self.list_object_ids_future = next_continuation_token.map(|token| {
                    ListObjectIdentifierFuture::new(
                        self.s3_client.clone(),
                        self.bucket.clone(),
                        Some(token),
                        self.max_keys,
                    )
                });
                return Poll::Ready(object_identifiers);
            }
            Err(()) => return Poll::Ready(None),
        }
    }
}

struct ListObjectIdentifierResult {
    /// A list of [`ObjectIdentifier`].
    object_identifiers: Option<Vec<ObjectIdentifier>>,
    /// A continuation token that can be used retrieve the next set of [`ObjectIdentifier`]s.
    /// The value is `None` if all object identifiers have been received.
    next_continuation_token: Option<String>,
}

struct ListObjectIdentifierFuture(
    Pin<Box<dyn Future<Output = Result<ListObjectIdentifierResult, ()>> + Send>>,
);

impl Future for ListObjectIdentifierFuture {
    type Output = Result<ListObjectIdentifierResult, ()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.get_mut().0.as_mut().poll(cx)
    }
}

impl ListObjectIdentifierFuture {
    /// Create a new [`ListObjectIdentifierFuture`] for the given bucket.
    fn new(
        s3_client: S3Client,
        bucket: String,
        continuation_token: Option<String>,
        max_keys: i64,
    ) -> Self {
        Self(Box::pin(async move {
            let req = ListObjectsV2Request {
                bucket: bucket.clone(),
                continuation_token: continuation_token.clone(),
                max_keys: Some(max_keys),
                ..Default::default()
            };

            let list_obj_resp = s3_client.list_objects_v2(req).await.map_err(|_| ())?;

            let next_continuation_token = S3Store::unpack_next_continuation_token(&list_obj_resp);
            let object_identifiers = S3Store::unpack_object_identifier(&list_obj_resp);

            Ok(ListObjectIdentifierResult {
                object_identifiers,
                next_continuation_token,
            })
        }))
    }
}

/// A stream that yields all object identifiers/keys of a bucket.
impl Stream for ListObjectsStream {
    type Item = Vec<ObjectIdentifier>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        trace!("Poll next object identifier");
        self.get_mut().poll_object_identifiers(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        coordinator::RoundSeed,
        crypto::{generate_integer, ByteObject},
        mask::{
            config::{BoundType, DataType, GroupType, MaskConfigs, ModelType},
            Mask,
            MaskedModel,
        },
        model::Model,
        MaskHash,
    };
    use futures::stream::{FuturesUnordered, StreamExt};
    use hex;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use rusoto_core::Region;
    use sodiumoxide::{crypto::hash::sha256, randombytes::randombytes};
    use std::{convert::TryFrom, iter, time::Instant};
    use tokio::task::JoinHandle;

    fn create_masked_model(byte_size: usize) -> (String, MaskedModel) {
        let mut prng = ChaCha20Rng::from_seed([0_u8; 32]);
        let config = MaskConfigs::from_parts(
            GroupType::Prime,
            DataType::F32,
            BoundType::B0,
            ModelType::M3,
        )
        .config();
        let integers = iter::repeat_with(|| generate_integer(&mut prng, config.order()))
            .take(10)
            .collect();
        (
            hex::encode(randombytes(32)),
            MaskedModel::from_parts(integers, config.clone()).unwrap(),
        )
    }

    fn create_global_model(byte_size: usize) -> (String, Model<i32>) {
        let mut rng = rand::thread_rng();
        (
            hex::encode(RoundSeed::generate().as_slice()),
            Model::try_from(
                (0..byte_size)
                    .map(|_| rng.gen_range(1, 21))
                    .collect::<Vec<i32>>(),
            )
            .unwrap(),
        )
    }

    fn create_minio_setup() -> Region {
        Region::Custom {
            name: String::from("eu-east-3"),
            endpoint: String::from("http://127.0.0.1:9000"),
        }
    }

    async fn create_client() -> S3Store {
        let region = create_minio_setup();
        let s3_store = S3Store::new(region);
        s3_store.clear_all().await.unwrap();
        s3_store.create_buckets().await.unwrap();
        s3_store
    }

    #[tokio::test]
    async fn test_upload_download_global_model() {
        let s3_store = create_client().await;

        for _ in 0..1100 {
            let (key, global_model) = create_global_model(1000);
            s3_store
                .upload_global_model(&key, &global_model)
                .await
                .unwrap();
        }

        s3_store
            .clear_bucket(s3_store.buckets.global_model())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_stream_masked_models() {
        let s3_store = create_client().await;

        for _ in 0..35 {
            let (key, masked_model) = create_masked_model(1_000_000);
            s3_store
                .upload_masked_model(&key, &masked_model)
                .await
                .unwrap();
        }

        let mut stream = s3_store.get_masked_model_identifier_stream();

        let mut futures =
            FuturesUnordered::<JoinHandle<Result<Option<MaskedModel>, StorageError>>>::new();
        while let Some(items) = stream.next().await {
            let len = items.len();
            for id in items {
                let store_clone = s3_store.clone();
                futures.push(tokio::spawn(async move {
                    let mask = store_clone.download_masked_model_id(id).await?;
                    Ok::<Option<MaskedModel>, StorageError>(mask)
                }));
            }

            let now = Instant::now();
            // wait for all the requests to finish
            loop {
                if futures.next().await.is_none() {
                    break;
                }
            }

            let new_now = Instant::now();
            println!(
                "downloaded {} masked models in {:?}",
                len,
                new_now.duration_since(now)
            );
        }
    }
}
