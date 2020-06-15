use std::task::Poll;

use futures::{
    future::{ready, Ready},
    task::Context,
};
use tower::Service;

use super::Sum2Request;
use crate::{
    coordinator::RoundParameters,
    crypto::ByteObject,
    message::{HeaderOwned, MessageOwned, PayloadOwned, Sum2Owned},
    services::{
        error::{RequestFailed, ServiceError},
        state_machine::StateMachineRequest,
    },
};

pub struct Sum2PreProcessorService;

impl Service<Sum2Request> for Sum2PreProcessorService {
    type Response = Result<StateMachineRequest, RequestFailed>;
    type Error = ServiceError;
    type Future = Ready<Result<Self::Response, ServiceError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, (header, message, params): Sum2Request) -> Self::Future {
        let pre_processor = Sum2PreProcessor {
            header,
            message,
            params,
        };
        ready(Ok(pre_processor.call()))
    }
}

struct Sum2PreProcessor {
    header: HeaderOwned,
    message: Sum2Owned,
    params: RoundParameters,
}

impl Sum2PreProcessor {
    fn call(self) -> Result<StateMachineRequest, RequestFailed> {
        if !self.has_valid_sum_signature() {
            return Err(RequestFailed::InvalidSumSignature);
        }
        if !self.is_eligible_for_sum_task() {
            return Err(RequestFailed::NotSumEligible);
        }

        let Self {
            header, message, ..
        } = self;
        Ok(MessageOwned {
            header,
            payload: PayloadOwned::Sum2(message),
        }
        .into())
    }
    /// Check whether this request contains a valid sum signature
    fn has_valid_sum_signature(&self) -> bool {
        let seed = &self.params.seed;
        let signature = &self.message.sum_signature;
        let pk = &self.header.participant_pk;
        pk.verify_detached(&signature, &[seed.as_slice(), b"sum"].concat())
    }

    /// Check whether this request comes from a participant that is eligible for the sum task.
    fn is_eligible_for_sum_task(&self) -> bool {
        self.message.sum_signature.is_eligible(self.params.sum)
    }
}
