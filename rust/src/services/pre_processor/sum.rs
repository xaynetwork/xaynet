use std::task::Poll;

use futures::{
    future::{ready, Ready},
    task::Context,
};
use tower::Service;

use super::SumRequest;
use crate::{
    coordinator::RoundParameters,
    crypto::ByteObject,
    message::{HeaderOwned, MessageOwned, PayloadOwned, SumOwned},
    services::{
        error::{RequestFailed, ServiceError},
        state_machine::StateMachineRequest,
    },
};

pub struct SumPreProcessorService;

impl Service<SumRequest> for SumPreProcessorService {
    type Response = Result<StateMachineRequest, RequestFailed>;
    type Error = ServiceError;
    type Future = Ready<Result<Self::Response, ServiceError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, (header, message, params): SumRequest) -> Self::Future {
        let pre_processor = SumPreProcessor {
            header,
            message,
            params,
        };
        ready(Ok(pre_processor.call()))
    }
}

struct SumPreProcessor {
    header: HeaderOwned,
    message: SumOwned,
    params: RoundParameters,
}

impl SumPreProcessor {
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
            payload: PayloadOwned::Sum(message),
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
