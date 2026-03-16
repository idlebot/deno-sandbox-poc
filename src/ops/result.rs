use deno_core::op2;
use deno_core::OpState;
use std::cell::RefCell;
use std::rc::Rc;

/// Key for storing the result in OpState.
pub struct ExecutionResult(pub Option<String>);

#[op2(fast)]
pub fn op_set_result(state: Rc<RefCell<OpState>>, #[string] value: String) {
    let mut state = state.borrow_mut();
    state.borrow_mut::<ExecutionResult>().0 = Some(value);
}
