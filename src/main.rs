extern crate myairdesk_lib;

use std::time::Duration;
use chrono::{Datelike, Days, Weekday};
use myairdesk_lib::myairdesk::{self};

#[tokio::main]
async fn main() {
    let client = myairdesk::Client::new();
    let client = match client.unlock().await {
        Ok(c) => { c },
        Err(_) => { println!("Error trying to authenticate user!"); return; },
    };

    loop {
        let now = chrono::Local::now();

        /* Get the next monday */
        let mut monday = now.clone();
        if now.weekday() as u8 > Weekday::Mon as u8 {
            monday = now.checked_add_days(Days::new(7 - now.weekday() as u64)).unwrap();
        }

        println!("Next week: {:?}", monday.date_naive().format("%Y-%m-%d").to_string());

        match client.week_bookings(&monday).await {
            Ok(bookings) => { println!("Bookings: {:?}", bookings); },
            Err(_) => { println!("Error getting week bookings."); },
        };
        
        println!("Booking next week!");

        /* Book new week */
        match client.book_week(&monday).await {
            Ok(_) => { println!("Booked week ({:?})", monday.date_naive().format("%Y-%m-%d").to_string()) },
            Err(_) => { println!("Error booking week ({:?})", monday.date_naive().format("%Y-%m-%d").to_string()) }
        }
            
        tokio::time::sleep(Duration::from_secs(1440)).await; // Each 24hrs
    }
}
