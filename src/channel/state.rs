/// Channel states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Closed,
    Errored,
    Joined,
    Joining,
    Leaving,
}
