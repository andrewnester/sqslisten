//! # SQSListen, a simple listener for AWS SQS queue.
//!
//! It allows you to set listener to your AWS SQS queue which will ask for the available messages in the queue and call the passed handler when the message received.
//! Once message received and processed (does not matter if handler returns error or not) the message is removed from the queue.
//!
//! ## Usage
//! ```rust
//! use sqslisten::{ReceiveMessageRequest, Region, SQSListen};
//! use std::{thread, time};
//!
//! fn main() {
//!     let mut sqs_listener = SQSListen::new(Region::UsEast1);
//!     let handle = sqs_listener.listen(
//!         ReceiveMessageRequest {
//!             queue_url: "<queue_url>".to_string(),
//!             ..ReceiveMessageRequest::default()
//!         },
//!         |msg, err| {
//!             match msg {
//!                 Some(message) => println!("Message received: {:?}", message),
//!                 None => {}
//!             }
//!
//!             match err {
//!                 Some(error) => println!("Error received: {:?}", error),
//!                 None => {}
//!             }
//!
//!             return Ok(());
//!         },
//!     );

//!     let ten_seconds = time::Duration::from_millis(100000);
//!     thread::sleep(ten_seconds);
//!
//!     handle.stop();
//! }
//! ```

extern crate rusoto_core;
extern crate rusoto_sqs;

pub use rusoto_core::{DispatchSignedRequest, ProvideAwsCredentials, Region, RusotoError};
pub use rusoto_sqs::{
    DeleteMessageRequest, Message, ReceiveMessageError, ReceiveMessageRequest, ReceiveMessageResult,
};

use clokwerk::{ScheduleHandle, Scheduler, TimeUnits};
use rusoto_sqs::{Sqs, SqsClient};
use std::option::Option;
use std::time::Duration;

#[derive(Clone)]
pub struct SQSListen {
    sqs_client: SqsClient,
    queue_url: String,
}

#[derive(Debug, Clone)]
pub struct HandlerError;

trait SQSListenHandler {
    fn handle() {}
}

impl SQSListen {
    pub fn new(region: Region) -> SQSListen {
        SQSListen {
            sqs_client: SqsClient::new(region),
            queue_url: "".to_string(),
        }
    }

    pub fn new_with<P, D>(
        request_dispatcher: D,
        credentials_provider: P,
        region: Region,
    ) -> SQSListen
    where
        P: ProvideAwsCredentials + Send + Sync + 'static,
        P::Future: Send,
        D: DispatchSignedRequest + Send + Sync + 'static,
        D::Future: Send,
    {
        SQSListen {
            sqs_client: SqsClient::new_with(request_dispatcher, credentials_provider, region),
            queue_url: "".to_string(),
        }
    }

    pub fn listen<F>(&mut self, input: ReceiveMessageRequest, handler: F) -> ScheduleHandle
    where
        F: Fn(
                Option<&Message>,
                Option<RusotoError<ReceiveMessageError>>,
            ) -> Result<(), HandlerError>
            + Send
            + Sync
            + 'static,
    {
        const DEFAULT_INTERVAL: u32 = 1;

        self.queue_url = input.queue_url.clone();
        let sqs_client = self.sqs_client.clone();
        let that = self.clone();

        let interval = match input.wait_time_seconds {
            Some(wait_time) => (wait_time as u32 + 1).seconds(),
            None => DEFAULT_INTERVAL.seconds(),
        };

        let mut scheduler = Scheduler::new();
        scheduler.every(interval).run(move || {
            match sqs_client.receive_message(input.clone()).sync() {
                Ok(response) => that.process_response(&response, &handler),
                Err(err) => {
                    let _ignored = handler(None, Some(err));
                }
            }
        });
        scheduler.watch_thread(Duration::from_millis(100))
    }

    fn process_response<F>(&self, response: &ReceiveMessageResult, handler: &F)
    where
        F: Fn(
                Option<&Message>,
                Option<RusotoError<ReceiveMessageError>>,
            ) -> Result<(), HandlerError>
            + Send
            + Sync
            + 'static,
    {
        match &response.messages {
            Some(messages) => {
                for message in messages {
                    let _ignored = handler(Some(&message), None);
                    self.ack_message(&message);
                }
            }
            None => {}
        }
    }

    fn ack_message(&self, message: &Message) {
        if message.receipt_handle.is_none() {
            return;
        }

        let _ignore = self
            .sqs_client
            .delete_message(DeleteMessageRequest {
                queue_url: self.queue_url.clone(),
                receipt_handle: message.receipt_handle.clone().unwrap(),
            })
            .sync();
    }
}
