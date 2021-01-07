use crate::{
    state_machine::{
        tests::utils::{shared_state, SelectFor},
        IntoPhase,
        MockIO,
        NewRound,
        Phase,
        State,
    },
    unwrap_step,
};

#[tokio::test]
async fn test_selected_for_sum() {
    let mut io = MockIO::new();
    io.expect_notify_sum().return_const(());
    let phase = make_phase(SelectFor::Sum, io);
    unwrap_step!(phase, complete, sum);
}

#[tokio::test]
async fn test_selected_for_update() {
    let mut io = MockIO::new();
    io.expect_notify_update().times(1).return_const(());
    io.expect_notify_load_model().times(1).return_const(());
    let phase = make_phase(SelectFor::Update, io);
    unwrap_step!(phase, complete, update);
}

#[tokio::test]
async fn test_not_selected() {
    let mut io = MockIO::new();
    io.expect_notify_idle().times(1).return_const(());
    let phase = make_phase(SelectFor::None, io);
    unwrap_step!(phase, complete, awaiting);
}

/// Instantiate a new round phase.
///
/// - `task` is the task we want the simulated participant to be selected for. If you want a
///   sum participant, pass `SelectedFor::Sum` for example.
/// - `io` is the mock the test wants to use. It should contains all the test expectations. The
///   reason for settings the mocked IO object in this helper is that once the phase is
///   created, `phase.io` is a `Box<dyn IO>`, not a `MockIO`. Therefore, it doesn't have any of
///   the mock methods (`expect_xxx()`, `checkpoint()`, etc.) so we cannot set any expectation
///   a posteriori
fn make_phase(task: SelectFor, io: MockIO) -> Phase<NewRound> {
    let shared = shared_state(task);

    // Check IntoPhase<NewRound> implementation
    let mut mock = MockIO::new();
    mock.expect_notify_new_round().times(1).return_const(());
    let mut phase: Phase<NewRound> =
        State::new(shared, Box::new(NewRound)).into_phase(Box::new(mock));

    // Set `phase.io` to the mock the test wants to use. Note that this drops the `mock` we
    // created above, so the expectations we set on `mock` run now.
    let _ = std::mem::replace(&mut phase.io, Box::new(io));
    phase
}
