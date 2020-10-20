#![cfg_attr(doc, forbid(warnings))]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input, Expr, Token,
};
struct Send {
    sender: Expr,
    metrics: Vec<Expr>,
}

impl Parse for Send {
    fn parse(input: ParseStream) -> Result<Self> {
        // metrics!(sender, metric_1);
        let sender = input.parse()?; // sender
        let mut metrics = Vec::new();

        // at least one metric is required, otherwise parse will fail.
        input.parse::<Token![,]>()?; // ,
        let metric = input.parse()?; // metric_1
        metrics.push(metric);

        // metrics!(sender, metric_1, metric_N);
        loop {
            if input.is_empty() {
                break;
            }

            input.parse::<Token![,]>()?; // ,
            let metric = input.parse()?; // metrics_N

            metrics.push(metric);
        }

        Ok(Send { sender, metrics })
    }
}

/// Allows one or multiple metrics to be sent through a `Sender` when the `metrics` feature is
/// enabled.
///
/// The idea is to only include the code for sending metrics if the `metrics` feature flag is enabled
/// during compilation. This can be achieved through conditional compilation, more precisely with the
/// attribute `cfg`.
///
/// See [here](https://www.worthe-it.co.za/programming/2018/11/18/compile-time-feature-flags-in-rust.html)
/// for more information about conditional compilation in Rust.
///
/// This macro helps to reduce the usage of the attribute `#[cfg(feature = "metrics")]` within the
/// source code.
///
/// ## Macro arguments:
///
/// `metrics!(sender, metric_1, metric_2, metric_N)`
///
/// ## Basic usage:
///
/// ```ignore
/// fn main() {
///     metrics!(sender, metrics::round::total_number::update(1));
///
///     metrics!(
///         sender,
///         metrics::round::total_number::update(1),
///         metrics::masks::total_number::update(1, 1, PhaseName::Idle)
///     );
/// }
/// ```
///
/// Equivalent code not using `metrics!`
///
/// ```ignore
/// fn main() {
///     #[cfg(feature = "metrics")]
///     {
///         sender.send(metrics::round::total_number::update(1)),
///     };
///
///     #[cfg(feature = "metrics")]
///     {
///         sender.send(metrics::round::total_number::update(1)),
///         sender.send(metrics::masks::total_number::update(1, 1, PhaseName::Idle)),
///     };
/// }
/// ```
///
/// ## Sender
///
/// A `Sender` must implement the method `pub fn send(&self, metrics: T)` where `T`
/// is the type of the metric.
///
/// ### Example of a `Sender` implementation
///
/// ```ignore
/// use influxdb::WriteQuery;
/// use tokio::sync::mpsc::Sender;
///
/// pub struct MetricsSender(Sender<WriteQuery>);
///
/// impl MetricsSender {
///     pub fn send(&mut self, query: WriteQuery) {
///         let _ = self.0.try_send(query).map_err(|e| error!("{}", e));
///     }
/// }
/// ```
#[proc_macro]
pub fn metrics(input: TokenStream) -> TokenStream {
    let Send { sender, metrics } = parse_macro_input!(input as Send);

    TokenStream::from(quote! {
            #[cfg(feature = "metrics")]
            {
                #(#sender.send(#metrics);)*
            }
    })
}
