// this did not work out right due to type signatures
// pub(crate) fn apply_rejection<RespT, Fut, T, ReqT, FutOut>(
//     handler: fn(T, ReqT) -> Fut,
// ) -> fn(T, ReqT) -> Fut
//     where Fut: Future<Output = Result<RespT, ErrorInfo>> + Send,
//     FutOut: Future<Output = Result<Result<RespT, ErrorInfo>, Rejection>> + Send,
// {
//     |x,y| handler(x,y).map(|t| {
//         let result: Result<Result<RespT, ErrorInfo>, Rejection> = Ok::<Result<RespT, ErrorInfo>, Rejection>(t);
//         result
//     })
// }
