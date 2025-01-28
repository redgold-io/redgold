use crate::DataStoreContext;
use redgold_schema::structs::{PriceSource, PriceTime, SupportedCurrency, Weighting};
use redgold_schema::RgResult;
use redgold_schema::SafeOption;

#[derive(Clone)]
pub struct PriceTimeStore {
    pub ctx: DataStoreContext
}

impl PriceTimeStore {

    pub async fn store_price_time(&self, price: f64, time: i64, currency: SupportedCurrency) -> RgResult<i64> {
        self.insert_price_time(PriceSource::OkxMinute, price, time, currency, SupportedCurrency::Usd).await
    }

    pub async fn insert_price_time_typed(&self, price_time: PriceTime) -> RgResult<i64> {
        let w: Weighting = price_time.price.safe_get()?.clone();
        self.insert_price_time(
            PriceSource::from_i32(price_time.source).ok_msg("price source")?,
            w.to_float(),
            price_time.time,
            SupportedCurrency::from_i32(price_time.currency).ok_msg("currency")?,
            SupportedCurrency::from_i32(price_time.denomination).ok_msg("denomination")?
        ).await
    }
    pub async fn insert_price_time(
        &self, price_source: PriceSource, price: f64, time: i64, currency: SupportedCurrency, denomination: SupportedCurrency)
        -> RgResult<i64> {

        let mut pool = self.ctx.pool().await?;

        let src = price_source as i32;
        let cur = currency as i32;
        let denom = denomination as i32;

        let rows = sqlx::query!(
            r#"
        INSERT OR REPLACE INTO price_time
        (source, currency, denomination, time, price)
        VALUES (
        ?1, ?2, ?3, ?4, ?5
        )"#,
            src, cur, denom, time, price
        )
            .execute(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.last_insert_rowid() as i64)
    }

    pub async fn select_price_time_range(&self, start: i64, end: i64) -> RgResult<Vec<PriceTime>> {
        let mut pool = self.ctx.pool().await?;
        let rows = sqlx::query!(
            r#"
        SELECT source, currency, denomination, time, price FROM price_time
        WHERE time >= ?1 AND time <= ?2
        "#,
            start, end
        )
            .fetch_all(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;

        let mut res = vec![];
        for x in rows_m {
            res.push(PriceTime {
                source: PriceSource::from_i32(x.source as i32).safe_get()?.clone() as i32,
                currency: SupportedCurrency::from_i32(x.currency as i32).safe_get()?.clone() as i32,
                denomination: SupportedCurrency::from_i32(x.denomination as i32).safe_get()?.clone() as i32,
                time: x.time,
                price: Some(Weighting::from_float_basis(x.price, 1e8 as i64))
            })
        }
        Ok(res)
    }

    pub async fn select_price(&self, time: i64, currency: SupportedCurrency) -> RgResult<Option<f64>> {
        let mut pool = self.ctx.pool().await?;
        let c = currency as i32;
        let u = SupportedCurrency::Usd as i32;
        let p = PriceSource::OkxMinute as i32;
        let rows = sqlx::query!(
            r#"
        SELECT price FROM price_time
        WHERE time = ?1 AND currency = ?2 AND denomination = ?3 AND source = ?4
        "#,
            time,
            c,
            u,
            p
        )
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.map(|x| x.price))
    }

    pub async fn max_time_price_by(&self, currency: SupportedCurrency, max_time: i64) -> RgResult<Option<f64>> {
        let mut pool = self.ctx.pool().await?;
        let c = currency as i32;
        let u = SupportedCurrency::Usd as i32;
        let p = PriceSource::OkxMinute as i32;
        let rows = sqlx::query!(
            r#"
        SELECT price FROM price_time
        WHERE currency = ?1 AND denomination = ?2 AND source = ?3 AND time <= ?4
        ORDER BY time DESC
        LIMIT 1
        "#,
            c,
            u,
            p,
            max_time
        )
            .fetch_optional(&mut *pool)
            .await;
        let rows_m = DataStoreContext::map_err_sqlx(rows)?;
        Ok(rows_m.map(|x| x.price))
    }
    //
    // pub async fn max_time_prices(&self, max_time: i64) -> RgResult<HashMap<SupportedCurrency, f64>> {
    //     let mut pool = self.ctx.pool().await?;
    //     let u = SupportedCurrency::Usd as i32;
    //     let p = PriceSource::OkxMinute as i32;
    //     let rows = sqlx::query!(
    //         r#"
    //     SELECT price, currency FROM price_time
    //     WHERE denomination = ?1 AND source = ?2 AND time <= ?
    //     ORDER BY time DESC
    //     LIMIT 1
    //     "#,
    //         u,
    //         p,
    //         max_time
    //     )
    //         .fetch_all(&mut *pool)
    //         .await;
    //     let rows_m = DataStoreContext::map_err_sqlx(rows)?;
    //     let mut hm = HashMap::new();
    //     for r in rows_m {
    //         let c = SupportedCurrency::from_i32(r.currency as i32).ok_msg("Missing currency")?;
    //         let p = r.price;
    //         hm.insert(c, p);
    //     }
    //     Ok(hm)
    // }

}