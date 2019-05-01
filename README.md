# SQSListen, a simple listener for AWS SQS queue.

[![Build Status](https://travis-ci.org/andrewnester/sqslisten.svg?branch=master)](https://travis-ci.org/andrewnester/sqslisten)

It allows you to set listener to your AWS SQS queue which will ask for the available messages in the queue and call the passed handler when the message received.
Once message received and processed (does not matter if handler returns error or not) the message is removed from the queue.

## Usage
```rust
use sqslisten::{ReceiveMessageRequest, Region, SQSListen};
use std::{thread, time};

fn main() {
    let mut sqs_listener = SQSListen::new(Region::UsEast1);
    let handle = sqs_listener.listen(
        ReceiveMessageRequest {
            queue_url: "<queue_url>".to_string(),
            ..ReceiveMessageRequest::default()
        },
        |msg, err| {
            match msg {
                Some(message) => println!("Message received: {:?}", message),
                None => {}
            }

            match err {
                Some(error) => println!("Error received: {:?}", error),
                None => {}
            }

            return Ok(());
        },
    );

    let ten_seconds = time::Duration::from_millis(100000);
    thread::sleep(ten_seconds);

    handle.stop();
}
```