use std::cell::RefCell;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

pub(crate) fn set(message: String) {
    LAST_ERROR.with(|last| *last.borrow_mut() = message);
}

pub(crate) fn get() -> Vec<u8> {
    LAST_ERROR.with(|last| last.borrow().as_bytes().to_vec())
}
