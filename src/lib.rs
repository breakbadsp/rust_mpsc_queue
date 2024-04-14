/* MpscQueue channel,
* Why MpscQueue => because MPMC has contention problem, it is kind of a bottleneck
* MpscQueue is easier to use and widely used.
* Design:
*   - APIs : MpscQueue::new 
        => this should create a channel and return tx and rx,
        tx => used to send data on a channel, can be cloned, will have enqueue API
        rx => used to received data on a channel, cannot be cloned, will have dequeue API to recieve data

    - Struct:
        buffer_:VecDeque<T> : this will be used as a buffer that will hold the data , (might need to wrap it in a mutex and then use arc to copy that mutex) we will see
*/

use std::{collections::VecDeque, sync::{Arc, Condvar, Mutex}};

struct Inner<T> {
    inner_: Mutex<VecDeque<T>>,
}

impl<T> Inner<T> {
    fn new() -> Self {
        Inner {
            inner_: Mutex::new(VecDeque::<T>::new()),
        }
    }
}

struct MpscQueue<T> {
    buffer_: Inner<T>,
    cv_: Condvar,
}

#[derive(Clone)]
pub struct Sender<T> {
    queue_: Arc<MpscQueue<T>>,
}

pub struct Receiver<T> {
    queue_: Arc<MpscQueue<T>>,
    cache_: VecDeque<T>,
}

impl<T> MpscQueue<T> {
    pub fn new() -> (Sender<T>, Receiver<T>) {
        let queue = Arc::new(MpscQueue{
            buffer_: Inner::<T>::new(),
            cv_: Condvar::new(),
        });

        let sender = Sender{
            queue_: Arc::clone(&queue),
        };

        let receiver = Receiver {
            queue_: Arc::clone(&queue),
            cache_: VecDeque::new(),
        };

        (sender, receiver)
    }

    fn push_back(&self, p_data: T) {
        {
            let mut lock_buffer = self.buffer_.lock().unwrap();
            lock_buffer.push_back(p_data);
        }
        self.cv_.notify_one();
    }

    fn pop_front(&self) -> Option<T> {
        let mut locked_buffer = self.buffer_.lock().unwrap();
        while locked_buffer.is_empty() {
            locked_buffer = self.cv_.wait(locked_buffer).unwrap();
        }
        locked_buffer.pop_front()
    }

}

impl<T> Sender<T> {
    pub fn enqueue(&self, p_data: T) {
        self.queue_.push_back(p_data)
    }
}

impl<T> Receiver<T> {
    pub fn dequeue(&self) -> Option<T> {
        if !self.cache_.is_empty() {
            return self.cache_.pop_front()
        }
        self.queue_.pop_front()
    }
}



#[cfg(test)]
mod tests {
    use core::{num, time};

    use crate::MpscQueue;

    #[test]
    fn spsc_test() {
        let (tx, rc) = MpscQueue::<i32>::new();
        
        let producer = std::thread::spawn({
            let tx_copy = tx.clone();
            println!("Producer started ");
            move || {
                for i in 0..100 {
                    println!("Sending {i}");
                    tx_copy.enqueue(i);
                }
            }
        });

        let consumer = std::thread::spawn({
            println!("Consumer started!");
            move || {
                loop {
                    let data_or_none = rc.dequeue();
                    match data_or_none {
                        None => {
                            return
                        }
                        Some(data) => {
                            println!("Received data {data}");
                        }
                    }
                }
            }
        });

        producer.join().unwrap();
        consumer.join().unwrap();
    }

    
    #[test]
    fn mpsc_test() {
        let (tx, rc) = MpscQueue::<i32>::new();

        let mut nums = vec![];
        let consumer = std::thread::spawn({
            println!("Consumer started : ");
            move || {
                loop {
                    let data_or_none = rc.dequeue();
                    match data_or_none {
                        None => {
                            println!("Producers is done sending, Terminating constumer");
                            return
                        }
                        Some(data) => {
                            println!("Received data: {data}");
                            nums.push(data);
                            println!("Total: {}", nums.len());
                        }
                    }
                }
            }
        });
        
        let mut producers = vec![];
        for i in 0..100 {
            let producer = std::thread::spawn({
                let tx_copy = tx.clone();
                println!("Producer started : ");
                move || {
                    println!("Sending {i}");
                    tx_copy.enqueue(i);
                    std::thread::sleep(time::Duration::from_millis(1));
                }
            });
            producers.push(producer);
        }

        for prod in producers {
            prod.join().unwrap();
        }
        //tx.enqueue(None);
        consumer.join().unwrap();
    }

}