use crate::binance::state::BinanceState;

impl BinanceState {
    //
    // making API request
    //
    // pub fn make_api_request(&mut self, endpoint: String) -> Option<Response> {
    //     //////////////////// BEGIN: RAW REQUESTS
    //     self.process_raw_requests_limit();
    //     //////////////////// END: RAW REQUESTS
    //
    //     //////////////////// BEGIN: REQUEST_WEIGHT
    //     self.process_request_weight_limit();
    //     //////////////////// END: REQUEST_WEIGHT
    //
    //     if self.api_last_code_received == 429 || self.api_last_code_received == 418 {
    //         let time_now = SystemTime::now();
    //         let problematic_request_delay = time_now
    //             .duration_since(self.api_last_problematic_request_time)
    //             .unwrap();
    //
    //         if problematic_request_delay.as_secs() <= self.api_retry_after_secs {
    //             warn!("still I have to wait as my back off time hasn't expired");
    //             self.api_requests_allowed = false;
    //         } else {
    //             warn!("back-off time expired - now I can make request again.");
    //             self.api_requests_allowed = true;
    //         }
    //     }
    //
    //     if self.api_requests_allowed {
    //         // info!(
    //         //     "Remaining weight: [{}], remaining raw count: [{}]",
    //         //     self.api_requests_weight_my_remaining_value,
    //         //     self.api_total_my_remaining_raw_requests_count
    //         // );
    //         self.api_requests_weight_my_remaining_value -= 1;
    //         self.api_total_my_remaining_raw_requests_count -= 1;
    //
    //         // info!(
    //         //     "Request executed. Remaining weight: [{}], remaining raw count: [{}]",
    //         //     self.api_requests_weight_my_remaining_value,
    //         //     self.api_total_my_remaining_raw_requests_count
    //         // );
    //
    //         let body = match reqwest::blocking::get(endpoint) {
    //             Ok(t) => t,
    //             Err(e) => {
    //                 error!("request error: {}", e);
    //                 return None;
    //             }
    //         };
    //
    //         // request returned 429?
    //         let status_code = body.status();
    //         if status_code == 429 || status_code == 418 {
    //             // default value - 5 minutes
    //             let mut retry_after_time_secs = DEFAULT_RETRY_VALUE_SECS;
    //
    //             // if let Some(sec) = body.headers().get("retry-after") {}
    //             match body.headers().get("retry-after") {
    //                 Some(sec) => {
    //                     if let Ok(value) = sec.to_str() {
    //                         if let Ok(v) = value.parse::<u64>() {
    //                             retry_after_time_secs = v;
    //                         }
    //                     }
    //                 }
    //                 None => (),
    //             }
    //
    //             warn!("status code 429 or 418 received: {status_code}");
    //             self.api_requests_allowed = false;
    //             self.api_last_code_received = status_code.as_u16();
    //             self.api_last_problematic_request_time = SystemTime::now();
    //             self.api_retry_after_secs = retry_after_time_secs;
    //             return None;
    //         }
    //
    //         self.api_last_code_received = status_code.as_u16();
    //
    //         // x-mbx-used-weight - to odjąć od wiadomej wartości
    //         match body.headers().get("x-mbx-used-weight") {
    //             Some(v) => {
    //                 // there is this header - so subtract
    //                 let val = v.to_str().unwrap();
    //                 let value = val.parse::<u64>().unwrap();
    //                 if self.api_requests_weight_my_remaining_value >= value {
    //                     self.api_requests_weight_my_remaining_value -= value
    //                 } else {
    //                     self.api_requests_weight_my_remaining_value = 0
    //                 }
    //             }
    //             None => {
    //                 // different request without weight - do nothing here
    //             }
    //         }
    //
    //         // info!("request returned: {:?}", body.text().unwrap());
    //         // info!("request was successfull");
    //         return Some(body);
    //     }
    //
    //     None
    // }
}
