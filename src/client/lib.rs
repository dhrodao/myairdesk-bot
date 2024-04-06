extern crate reqwest;
extern crate serde;

pub mod myairdesk {
    use base64::prelude::*;
    use std::collections::HashMap;
    use std::env;
    use chrono::{DateTime, TimeZone, Days};

    const AUTH_ENDPOINT: &str = "https://www.myairdesk.com/bertrandt/api/Login/LoginWithUsername";

    #[derive(Debug)]
    #[allow(unused, non_camel_case_types)]
    pub enum BookingError {
        NO_ERROR,
        HTTP_ERROR,
        BOOK_ERROR,
        PAYLOAD_ERROR,
    }

    #[derive(Debug, Default)]
    struct Context {
        env_vars: HashMap<String, String>,
        user_details: UserDetails,
    }

    #[derive(Debug, Default, serde::Deserialize)]
    #[allow(unused)]
    struct Algorithm {
        alg: String,
        typ: String,
    }

    #[derive(Debug, Default, serde::Deserialize)]
    #[allow(unused, non_snake_case)]
    struct UserDetails {
        userId: String,
        userRoleId: String,
        expTime: String,
        nbf: u64,
        exp: u64,
        iat: u64,
    }
    
    #[derive(serde::Serialize)]
    struct CredentialsPayload {
        username: String,
        password: String,
        save_credentials: bool,
    }

    #[derive(Debug, serde::Serialize)]
    #[allow(unused, non_snake_case)]
    struct BookingPayload {
        id: u64,
        userId: u64,
        date: String,
        workplaceId: u64,
        bookedById: u64,
    }

    #[derive(Debug, serde::Deserialize)]
    struct SessionToken {
        token: String,
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(unused)]
    struct Response {
        data: SessionToken,
        message: String,
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(non_snake_case, unused)]
    pub struct Book {
        pub date: String,
        pub bookingOfficeSectorName: String,
        pub bookingWorkplaceName: String,
    }

    #[derive(Default, Debug, serde::Deserialize)]
    #[allow(unused)]
    pub struct Bookings {
        pub bookings: Vec<Book>,
    }

    pub struct Locked;
    pub struct Unlocked;

    pub struct Client<State = Locked> {
        ctx: Context,
        http_client: reqwest::Client,
        token: String,
        state: std::marker::PhantomData<State>,
    }

    macro_rules! unwrap_or_return_err {
        ( $e:expr ) => {
            match $e {
                Ok(x) => x,
                Err(_) => return Err(()),
            }
        }
    }
    
    impl Client<Locked> {
        pub async fn unlock(mut self) -> Result<Client<Unlocked>, ()> {

            match self.get_env_vars() {
                Some(v) => { self.ctx.env_vars = v },
                None => { return Err(()) },
            };

            let payload = CredentialsPayload {
                username: self.ctx.env_vars.get("AIRDESK_USER").unwrap().to_owned(),
                password: self.ctx.env_vars.get("AIRDESK_PASS").unwrap().to_owned(),
                save_credentials: true,
            };

            let payload_bytes = unwrap_or_return_err!(serde_json::to_string(&payload));
            
            let response = unwrap_or_return_err!(self.http_client.post(AUTH_ENDPOINT)
                .body(payload_bytes)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .send()
                .await);

            let response_str = unwrap_or_return_err!(response.text().await);

            let parsed_response: Response = unwrap_or_return_err!(serde_json::from_str(response_str.clone().as_str()));
            self.token = parsed_response.data.token;

            self.ctx.user_details = unwrap_or_return_err!(self.decode_token());

            Ok(Client { ctx: self.ctx,
                        http_client: self.http_client,
                        token: self.token,
                        state: std::marker::PhantomData })
        }

        fn get_env_vars(&self) -> Option<HashMap<String, String>> {
            let mut vars: HashMap<String, String> = HashMap::new();
            let names: Vec<&str> = ["AIRDESK_USER", "AIRDESK_PASS", "AIRDESK_WORKPLACE"].to_vec();

            names.iter().for_each(|&key| {
                match env::var(key) {
                    Ok(value) => { vars.insert(key.to_string(), value); },
                    Err(_) => {},
                }
            });

            if vars.len() != names.len() {
                return None;
            }

            Some(vars)
        }
        
        fn decode_token(&self) -> Result<UserDetails, ()> {
            let v: Vec<&str> = self.token.split('.').collect();
            if v.len() != 3
            {
                return Err(());
            }

            let mut iter = v.iter();
            let decoded = unwrap_or_return_err!(BASE64_STANDARD_NO_PAD.decode(iter.next().unwrap()));
            let decoded = String::from_utf8_lossy(&decoded);
            let _algorithm: Algorithm = unwrap_or_return_err!(serde_json::from_str(&decoded));
            
            let decoded = unwrap_or_return_err!(BASE64_STANDARD_NO_PAD.decode(iter.next().unwrap()));
            let decoded = String::from_utf8_lossy(&decoded);
            let user_details: UserDetails = unwrap_or_return_err!(serde_json::from_str(&decoded));

            // The last item is not needed

            Ok(user_details)
        }
    }

    impl Client<Unlocked> {
        pub fn lock(self) -> Client<Locked> {
        Client { ctx: Default::default(),
                 http_client: self.http_client,
                 token: Default::default(),
                 state: std::marker::PhantomData }
        }

        pub async fn week_bookings<Tz>(&self, now: &DateTime<Tz>) -> Result<Bookings, ()>
            where Tz: TimeZone {
            let unix_timestamp = now.timestamp_millis();
            let bearer_token = String::from(format!("Bearer {}", self.token));

            let response = unwrap_or_return_err!(
                    self.http_client
                        .get(format!("https://www.myairdesk.com/bertrandt/api/Bookings/GetWeekBookingsForUser?userId={}&mondayDateUnixStamp={}",
                                     self.ctx.user_details.userId, unix_timestamp))
                        .header(reqwest::header::AUTHORIZATION, bearer_token)
                        .send()
                        .await
                    );

            let response_text = unwrap_or_return_err!(response.text().await);
            let weekly_bookings: Vec<Book> = serde_json::from_str(&response_text).unwrap_or_default();
            let bookings = Bookings {
                bookings: weekly_bookings
            };
    
            Ok(bookings)
        }

        pub async fn book_day(&self, day_millis: i64) -> Result<(), BookingError> {
            let bearer_token = String::from(format!("Bearer {}", self.token));

            let booking_payload = BookingPayload {
                id: 0,
                userId: self.ctx.user_details.userId.parse::<u64>().unwrap(),
                bookedById: self.ctx.user_details.userId.parse::<u64>().unwrap(),
                date: day_millis.to_string(),
                workplaceId: self.ctx.env_vars.get("AIRDESK_WORKPLACE")
                                              .unwrap()
                                              .parse::<u64>()
                                              .unwrap()
            };
            let payload_bytes = match serde_json::to_string(&booking_payload) {
                Ok(p) => { p },
                Err(_) => { return Err(BookingError::PAYLOAD_ERROR) },
            };
            
            let response = match self.http_client
                                .post("https://www.myairdesk.com/bertrandt/api/Bookings")
                                .body(payload_bytes)
                                .header(reqwest::header::CONTENT_TYPE, "application/json")
                                .header(reqwest::header::AUTHORIZATION, bearer_token)
                                .send()
                                .await {
                Ok(r) => { r },
                Err(_) => { return Err(BookingError::HTTP_ERROR) },
            };

            match response.error_for_status() {
                Ok(_) => { return Ok(()) },
                Err(_) => { return Err(BookingError::BOOK_ERROR) },
            }
        }

        pub async fn book_week<Tz>(&self, day: &DateTime<Tz>) -> Result<(), BookingError>
        where Tz: TimeZone {
            for i in 0..5 {
                let day = day.clone()
                                           .checked_add_days(Days::new(i))
                                           .unwrap();
                let day_str = day.date_naive().format("%Y-%m-%d").to_string();
                println!("Booking: {:?}", day_str);

                match self.book_day(day.timestamp_millis()).await {
                    Err(BookingError::HTTP_ERROR) => { println!("({:?}) Error booking day {:?}.", BookingError::HTTP_ERROR, day_str); return Err(BookingError::HTTP_ERROR); },
                    Err(BookingError::BOOK_ERROR) => { println!("({:?}) Error booking day {:?}, it may be already booked.", BookingError::BOOK_ERROR, day_str) },
                    _ => {},
                };
            }

            Ok(())
        }
    }

    impl Client {
        pub fn new() -> Client<Locked> {
            Client {
                ctx: Context::default(),
                http_client: reqwest::Client::builder()
                                .cookie_store(true)
                                .build()
                                .unwrap(),
                token: Default::default(),
                state: Default::default()
            }
        }
    }
}