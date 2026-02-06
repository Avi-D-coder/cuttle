#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetKind {
    Point,
    Royal,
    Jack,
    Joker,
}
