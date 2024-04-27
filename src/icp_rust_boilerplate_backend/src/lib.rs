#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Event {
    id: u64,
    title: String,
    date: String,
    time: String,
    max_attendant: u32,
    attachment_url: String,
    created_at: u64,
    updated_at: Option<u64>,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Event {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Event {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Event, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct EventPayload {
    title: String,
    date: String,
    time: String,
    max_attendant: u32,
    attachment_url: String,
}

#[ic_cdk::query]
fn get_event(id: u64) -> Result<Event, Error> {
    match _get_event(&id) {
        Some(event) => Ok(event),
        None => Err(Error::NotFound {
            msg: format!("an event with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_event(event: EventPayload) -> Option<Event> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let event = Event {
        id,
        title: event.title,
        date: event.date,
        time: event.time,
        max_attendant: event.max_attendant,
        attachment_url: event.attachment_url,
        created_at: time(),
        updated_at: None,
    };
    ic_cdk:: println!("Successfully create new event {}.", event.title);
    do_insert(&event);
    Some(event)
}

#[ic_cdk::update]
fn update_event(id: u64, payload: EventPayload) -> Result<Event, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut event) => {
            event.title = payload.title;
            event.date = payload.date;
            event.time = payload.time;
            event.max_attendant = payload.max_attendant;
            event.attachment_url = payload.attachment_url;
            event.updated_at = Some(time());
            do_insert(&event);
            Ok(event)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update an event with id={}. event not found",
                id
            ),
        }),
    }
}

// helper method to perform insert.
fn do_insert(event: &Event) {
    STORAGE.with(|service| service.borrow_mut().insert(event.id, event.clone()));
}

#[ic_cdk::update]
fn delete_event(id: u64) -> Result<Event, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(event) => Ok(event),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete an event with id={}. event not found.",
                id
            ),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// a helper method to get a message by id. used in get_message/update_message
fn _get_event(id: &u64) -> Option<Event> {
    STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();