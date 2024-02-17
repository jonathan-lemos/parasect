/// Instantiation of this struct outputs a stream of logs to stdout.
///
/// There should be no more than one instance of either `NoTtyUi` or `TtyUi` at any time.
///
/// The logs stops outputting when this struct is dropped.
pub struct NoTtyUi {}
