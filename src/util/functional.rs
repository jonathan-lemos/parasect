pub fn compose_once<A, B, C, F: FnOnce(B) -> C, G: FnOnce(A) -> B>(
    f: F,
    g: G,
) -> impl FnOnce(A) -> C {
    |x| f(g(x))
}
