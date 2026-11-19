use std::cell::RefCell;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
    /// Encoded `Refusal` of the last failure when it was a rejected command, otherwise empty.
    static LAST_REFUSAL: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Records a failure; `refusal` is the encoded `Refusal` when a command was rejected.
pub(crate) fn set(message: String, refusal: Vec<u8>) {
    LAST_ERROR.with(|last| *last.borrow_mut() = message);
    LAST_REFUSAL.with(|last| *last.borrow_mut() = refusal);
}

pub(crate) fn get() -> Vec<u8> {
    LAST_ERROR.with(|last| last.borrow().as_bytes().to_vec())
}

pub(crate) fn refusal() -> Vec<u8> {
    LAST_REFUSAL.with(|last| last.borrow().clone())
}
