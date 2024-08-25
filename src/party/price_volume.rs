use curv::arithmetic::Zero;
use rocket::serde::{Deserialize, Serialize};
use log::error;
use crate::party::central_price::DUST_LIMIT;


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PriceVolume {
    pub price: f64, // RDG/BTC (in satoshis for both) for now
    pub volume: u64, // Volume of RDG available
}

impl PriceVolume {

    pub fn generate(
        available_volume: u64,
        center_price: f64,
        divisions: i32,
        price_width: f64,
        scale: f64
    ) -> Vec<PriceVolume> {

        if available_volume < DUST_LIMIT as u64 {
            return vec![];
        }

        let divisions_f64 = divisions as f64;

        // Calculate the common ratio
        let ratio = (1.0 / scale).powf(1.0 / (divisions_f64 - 1.0));

        // Calculate the first term
        let first_term = available_volume as f64 * scale / (1.0 - ratio.powf(divisions_f64));

        let mut price_volumes = Vec::new();

        for i in 0..divisions {
            let price_offset = (i+1) as f64;
            let price = center_price + (price_offset * (price_width/divisions_f64));
            if price.is_nan() || price.is_infinite()  || price.is_sign_negative() || price.is_zero() {
                // error!("Price is invalid: {} center_price: {} price_offset: {} price_width: {} divisions_f64: {}",
                //        price, center_price, price_offset, price_width, divisions_f64);
            } else {
                let multiplier = ratio.powi(divisions - i);
                let multiplier = f64::sqrt(multiplier);
                let volume = (first_term * multiplier) as u64;
                price_volumes.push(PriceVolume { price, volume });
            }
        }

        // Normalize the volumes so their sum equals available_volume
        Self::normalize_volumes(available_volume, &mut price_volumes);


// Re-calculate the total after normalization
        let adjusted_total_volume: u64 = price_volumes.iter().map(|pv| pv.volume).sum();

        // Adjust volumes to ensure total equals available_volume
        let mut adjustment = available_volume as i64 - adjusted_total_volume as i64;
        for pv in &mut price_volumes {
            if adjustment == 0 {
                break;
            }

            if adjustment > 0 && pv.volume > 0 {
                pv.volume += 1;
                adjustment -= 1;
            } else if adjustment < 0 && pv.volume > 1 {
                pv.volume -= 1;
                adjustment += 1;
            }
        }

        // Final assert
        let final_total_volume: u64 = price_volumes.iter().map(|pv| pv.volume).sum();
        assert!(final_total_volume <= available_volume, "Total volume should equal available volume or be less than");


        //
        // let total_volume = price_volumes.iter().map(|v| v.volume).sum::<u64>();
        //
        // // Normalize the volumes so their sum equals available_volume
        // for pv in &mut price_volumes {
        //     pv.volume = ((pv.volume as f64 / total_volume as f64) * available_volume as f64) as u64;
        // }
        //
        // if total_volume != available_volume {
        //     let delta = total_volume as i64 - available_volume as i64;
        //     if let Some(last) = price_volumes.last_mut() {
        //         if delta > 0 && (last.volume as u64) > delta as u64 {
        //             last.volume = ((last.volume as i64) - delta) as u64;
        //         } else if delta < 0 {
        //             last.volume = ((last.volume as i64) - delta) as u64;
        //         }
        //     }
        // }
        //
        // let total_volume = price_volumes.iter().map(|v| v.volume).sum::<u64>();
        // assert_eq!(total_volume, available_volume, "Total volume should equal available volume");

        let mut fpv = vec![];

        for pv in price_volumes {
            if pv.volume <= 0 || pv.volume > available_volume {
                error!("Volume is invalid: {:?}", pv);
            } else {
                fpv.push(pv);
            }
        }
        fpv
    }
    //
    // fn normalize_volumes(available_volume: u64, price_volumes: &mut Vec<PriceVolume>) {
    //     let current_total_volume: u64 = price_volumes.iter().map(|pv| pv.volume).sum();
    //     for pv in price_volumes.iter_mut() {
    //         pv.volume = ((pv.volume as f64 / current_total_volume as f64) * available_volume as f64).round() as u64;
    //     }
    // }

    fn normalize_volumes(available_volume: u64, price_volumes: &mut Vec<PriceVolume>) {
        let current_total_volume: u64 = price_volumes.iter().map(|pv| pv.volume).sum();

        // Initially normalize volumes
        for pv in price_volumes.iter_mut() {
            pv.volume = ((pv.volume as f64 / current_total_volume as f64) * available_volume as f64).round() as u64;
        }

        let mut dust_trigger = false;

        for pv in price_volumes.iter_mut() {
            if pv.volume < DUST_LIMIT as u64 {
                dust_trigger = true;
            }
        }

        if dust_trigger {
            let mut new_price_volumes = vec![];
            let divs = (available_volume / DUST_LIMIT as u64) as usize;
            for (i,pv) in price_volumes.iter_mut().enumerate() {
                if i < divs {
                    new_price_volumes.push(PriceVolume {
                        price: pv.price,
                        volume: DUST_LIMIT as u64
                    });
                }
            }
            price_volumes.clear();
            price_volumes.extend(new_price_volumes);
        }
    }

}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PriceVolumeBroken {
    pub price: Option<f64>, // RDG/BTC (in satoshis for both) for now
    pub volume: Option<u64>, // Volume of RDG available
}
