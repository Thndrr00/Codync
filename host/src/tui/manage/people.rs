//! People on this computer, as a terminal sheet: who uses it besides its owner, the paired devices
//! each person uses, and giving a device to a person or back to the owner (`docs/features/people.md`).
//!
//! The host answers `people`, `addPerson`, `updatePerson`, `removePerson`, `setDevicePerson` and
//! `devices` to the owner of this computer and refuses a member's phone. Removing a person is the
//! only destructive-looking action, and it deletes nothing: their devices, bots, chats and connectors
//! become the owner's.

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::super::app::{App, Confirm, ConfirmAct, Editor, Overlay};
use super::super::l10n::{fill, tr};
use super::{Fields, Input, Reply, Submit};

/// Which of the sheet's two lists has the cursor.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PeopleSection {
    #[default]
    People,
    Devices,
}

/// The people on this computer and its paired devices, with the person each device acts as.
#[derive(Default)]
pub struct PeopleSheet {
    /// `people`; `None` until the host answers.
    pub people: Option<Vec<Value>>,
    /// `devices`; `None` until the host answers.
    pub devices: Option<Vec<Value>>,
    pub section: PeopleSection,
    pub cursor: usize,
    pub step: PeopleStep,
    pub busy: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub enum PeopleStep {
    #[default]
    List,
    /// The person picker for one device.
    Assign(PersonAssign),
}

/// A device's assignment being picked: this computer's owner first, then every person.
pub struct PersonAssign {
    /// The device's key (`setDevicePerson` names it).
    pub device: String,
    /// The device's name, for the sheet.
    pub name: String,
    /// The person it acts as now; empty is this computer's owner.
    pub current: String,
    /// `(person id, name)`; the empty id is this computer's owner.
    pub choices: Vec<(String, String)>,
    pub cursor: usize,
}

impl PeopleSheet {
    pub fn people(&self) -> &[Value] {
        self.people.as_deref().unwrap_or_default()
    }

    pub fn devices(&self) -> &[Value] {
        self.devices.as_deref().unwrap_or_default()
    }

    /// The rows of the list the cursor is in.
    pub fn rows(&self) -> &[Value] {
        match self.section {
            PeopleSection::People => self.people(),
            PeopleSection::Devices => self.devices(),
        }
    }

    pub fn current(&self) -> Option<&Value> {
        self.rows().get(self.cursor)
    }

    /// Takes `people`'s answer, keeping the cursor on a row that still exists.
    pub fn fill_people(&mut self, value: &Value) {
        self.people = Some(value["people"].as_array().cloned().unwrap_or_default());
        self.cursor = self.cursor.min(self.rows().len().saturating_sub(1));
        self.error = None;
    }

    /// Takes `devices`' answer, keeping the cursor on a row that still exists.
    pub fn fill_devices(&mut self, value: &Value) {
        self.devices = Some(value["devices"].as_array().cloned().unwrap_or_default());
        self.cursor = self.cursor.min(self.rows().len().saturating_sub(1));
        self.busy = false;
        self.error = None;
    }

    /// The names of the devices assigned to `person` (empty: this computer's owner), in the order
    /// the host lists them.
    pub fn devices_of(&self, person: &str) -> Vec<String> {
        self.devices()
            .iter()
            .filter(|device| device["personId"].as_str().unwrap_or_default() == person)
            .filter_map(|device| device["name"].as_str().map(str::to_owned))
            .collect()
    }

    /// The picker for `device`; the cursor starts on the person it acts as now.
    pub fn assign(&self, device: &Value) -> PersonAssign {
        let current = device["personId"].as_str().unwrap_or_default();
        let mut choices = vec![(String::new(), tr("This computer's owner").into_owned())];
        choices.extend(
            self.people()
                .iter()
                .filter_map(|person| Some((person["id"].as_str()?.to_owned(), person["name"].as_str()?.to_owned()))),
        );
        PersonAssign {
            device: device["key"].as_str().unwrap_or_default().to_owned(),
            name: device["name"].as_str().unwrap_or_default().to_owned(),
            current: current.to_owned(),
            cursor: choices.iter().position(|(id, _)| id == current).unwrap_or(0),
            choices,
        }
    }
}

/// The form that adds a person (`id` `None`) or renames one and changes their email.
pub fn person_fields(id: Option<&str>, name: &str, email: &str) -> Fields {
    let title = match id {
        Some(name) => fill(&tr("Rename {name}"), &[("name", name)]),
        None => tr("Add a person").into_owned(),
    };
    Fields {
        title,
        note: tr("Someone who uses this computer with their own phone or their own bots.").into_owned(),
        choices: vec![],
        choice: 0,
        inputs: vec![
            Input {
                name: "name".into(),
                label: "Name".into(),
                secret: false,
                required: true,
                multiline: false,
                placeholder: String::new(),
                ed: Editor::with(name),
            },
            Input {
                name: "email".into(),
                label: "Email (optional)".into(),
                secret: false,
                required: false,
                multiline: false,
                placeholder: "person@example.com".into(),
                ed: Editor::with(email),
            },
        ],
        cursor: 0,
        error: None,
        saving: false,
        submit: Submit::Person { id: id.map(str::to_owned) },
    }
}

/// The body `addPerson` or `updatePerson` takes from a filled form: the name, and the email only
/// when one was typed, so clearing it on an edited person clears theirs.
pub fn person_body(id: Option<&str>, values: &serde_json::Map<String, Value>) -> Value {
    let mut body = json!({"name": values.get("name").and_then(Value::as_str).unwrap_or_default()});
    if let Some(email) = values.get("email").and_then(Value::as_str).filter(|email| !email.is_empty()) {
        body["email"] = json!(email);
    }
    if let Some(id) = id {
        body["id"] = json!(id);
    }
    body
}

impl App {
    pub(in crate::tui) fn open_people(&mut self) {
        self.overlays.push(Overlay::People(Box::default()));
        self.load_people();
    }

    /// Loads the people and the paired devices into the open People sheet.
    pub(in crate::tui) fn load_people(&mut self) {
        self.sheet("people", json!({}), Reply::People);
        self.sheet("devices", json!({}), Reply::PeopleDevices);
    }

    pub(in crate::tui) fn people_key(&mut self, mut sheet: Box<PeopleSheet>, k: KeyEvent) -> Option<Overlay> {
        let step = std::mem::take(&mut sheet.step);
        sheet.step = match step {
            PeopleStep::Assign(assign) => Self::assign_key(self, &mut sheet, assign, k),
            PeopleStep::List => return Self::people_list_key(self, sheet, k),
        };
        Some(Overlay::People(sheet))
    }

    fn people_list_key(app: &mut Self, mut sheet: Box<PeopleSheet>, k: KeyEvent) -> Option<Overlay> {
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return None,
            KeyCode::Tab | KeyCode::BackTab => {
                sheet.section = match sheet.section {
                    PeopleSection::People => PeopleSection::Devices,
                    PeopleSection::Devices => PeopleSection::People,
                };
                sheet.cursor = sheet.cursor.min(sheet.rows().len().saturating_sub(1));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                sheet.cursor = (sheet.cursor + 1).min(sheet.rows().len().saturating_sub(1));
            }
            KeyCode::Up | KeyCode::Char('k') => sheet.cursor = sheet.cursor.saturating_sub(1),
            // Loads again: somebody paired a device, or another app changed the list.
            KeyCode::Char('r') => {
                sheet.people = None;
                sheet.devices = None;
                app.load_people();
            }
            KeyCode::Char('n') => {
                app.overlays.push(Overlay::Fields(Box::new(person_fields(None, "", ""))));
            }
            KeyCode::Enter if sheet.section == PeopleSection::People => {
                if let Some(person) = sheet.current().cloned() {
                    let id = person["id"].as_str().unwrap_or_default().to_owned();
                    let name = person["name"].as_str().unwrap_or_default();
                    let email = person["email"].as_str().unwrap_or_default();
                    app.overlays.push(Overlay::Fields(Box::new(person_fields(Some(&id), name, email))));
                }
            }
            KeyCode::Enter | KeyCode::Char('a') if sheet.section == PeopleSection::Devices => {
                if let Some(device) = sheet.current().cloned() {
                    sheet.step = PeopleStep::Assign(sheet.assign(&device));
                }
            }
            KeyCode::Char('x' | 'd') if sheet.section == PeopleSection::People => {
                if let Some(person) = sheet.current().cloned() {
                    let id = person["id"].as_str().unwrap_or_default().to_owned();
                    let name = person["name"].as_str().unwrap_or_default();
                    let confirm = Confirm {
                        title: fill(&tr("Remove {name}?"), &[("name", name)]),
                        detail: "Their phone, bots, chats and connectors become yours. Nothing is deleted.".into(),
                        note: "Connectors become shared, so every bot on this computer can use them.".into(),
                        button: "Remove",
                        act: ConfirmAct::RemovePerson(id),
                    };
                    app.overlays.push(Overlay::Confirm(confirm));
                }
            }
            _ => {}
        }
        Some(Overlay::People(sheet))
    }

    fn assign_key(app: &mut Self, sheet: &mut PeopleSheet, mut assign: PersonAssign, k: KeyEvent) -> PeopleStep {
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => PeopleStep::List,
            KeyCode::Down | KeyCode::Char('j') => {
                assign.cursor = (assign.cursor + 1).min(assign.choices.len().saturating_sub(1));
                PeopleStep::Assign(assign)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                assign.cursor = assign.cursor.saturating_sub(1);
                PeopleStep::Assign(assign)
            }
            KeyCode::Enter => {
                let person = assign.choices.get(assign.cursor).map(|(id, _)| id.clone()).unwrap_or_default();
                sheet.busy = true;
                sheet.error = None;
                let person = if person.is_empty() { Value::Null } else { Value::String(person) };
                app.sheet("setDevicePerson", json!({"device": assign.device, "person": person}), Reply::PeopleSaved);
                PeopleStep::List
            }
            _ => PeopleStep::Assign(assign),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::test_app;
    use crossterm::event::{KeyCode, KeyEvent};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    fn filled() -> PeopleSheet {
        let mut sheet = PeopleSheet::default();
        sheet.fill_people(&json!({"people": [
            {"id": "ana", "name": "Ana", "email": "ana@example.com"},
            {"id": "ben", "name": "Ben"}
        ]}));
        sheet.fill_devices(&json!({"devices": [
            {"key": "phone", "name": "iPhone", "personId": "ana", "person": "Ana"},
            {"key": "pad", "name": "iPad"}
        ]}));
        sheet
    }

    #[test]
    fn rows_list_each_person_and_the_devices_that_act_as_them() {
        let sheet = filled();
        assert_eq!(sheet.devices_of("ana"), ["iPhone"]);
        assert!(sheet.devices_of("ben").is_empty());
        assert_eq!(sheet.devices_of(""), ["iPad"], "devices with no person are the owner's");
        assert_eq!(sheet.rows().len(), 2, "the people list is the one the cursor starts in");
    }

    #[test]
    fn the_picker_offers_the_owner_then_every_person_and_starts_on_the_current_one() {
        let sheet = filled();
        let assign = sheet.assign(&json!({"key": "phone", "name": "iPhone", "personId": "ben"}));
        let labels: Vec<&str> = assign.choices.iter().map(|(_, name)| name.as_str()).collect();
        assert_eq!(labels, ["This computer's owner", "Ana", "Ben"]);
        assert_eq!((assign.cursor, assign.current.as_str()), (2, "ben"));
        assert_eq!(sheet.assign(&json!({"key": "pad", "name": "iPad"})).cursor, 0);
    }

    #[test]
    fn a_person_form_carries_the_name_and_only_the_email_that_was_typed() {
        let add = person_fields(None, "", "");
        assert!(matches!(&add.submit, Submit::Person { id: None }));
        assert_eq!(add.inputs.len(), 2);

        let edit = person_fields(Some("ana"), "Ana", "ana@example.com");
        assert!(matches!(&edit.submit, Submit::Person { id: Some(id) } if id == "ana"));
        assert_eq!(edit.inputs[1].ed.text, "ana@example.com");

        let values = json!({"name": "Ana", "email": "ana@example.com"}).as_object().cloned().unwrap();
        assert_eq!(person_body(None, &values), json!({"name": "Ana", "email": "ana@example.com"}));

        let values = json!({"name": "Ana"}).as_object().cloned().unwrap();
        assert_eq!(
            person_body(Some("ana"), &values),
            json!({"name": "Ana", "id": "ana"}),
            "an edited person with no email clears theirs"
        );
    }

    #[test]
    fn the_sheet_switches_lists_and_opens_the_form_removal_and_picker() {
        let mut app = test_app();
        app.overlays.push(Overlay::People(Box::new(filled())));

        // Tab reaches the devices, where Enter opens the person picker.
        app.on_key(key(KeyCode::Tab));
        assert!(matches!(&app.overlays.last(), Some(Overlay::People(p)) if p.section == PeopleSection::Devices));
        app.on_key(key(KeyCode::Enter));
        let Some(Overlay::People(p)) = app.overlays.last() else { panic!("the sheet is open") };
        assert!(matches!(&p.step, PeopleStep::Assign(a) if a.name == "iPhone"));
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(&app.overlays.last(), Some(Overlay::People(p)) if matches!(p.step, PeopleStep::List)));

        // n adds a person, in a fields sheet over the list.
        app.on_key(key(KeyCode::Char('n')));
        assert!(
            matches!(&app.overlays.last(), Some(Overlay::Fields(f)) if matches!(&f.submit, Submit::Person { id: None }))
        );
        app.on_key(key(KeyCode::Esc));

        // Back in the people list, x asks before removing.
        app.on_key(key(KeyCode::Tab));
        app.on_key(key(KeyCode::Char('x')));
        assert!(
            matches!(&app.overlays.last(), Some(Overlay::Confirm(c)) if matches!(&c.act, ConfirmAct::RemovePerson(id) if id.as_str() == "ana"))
        );
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(&app.overlays.last(), Some(Overlay::People(_))), "cancelling returns to the list");
        app.on_key(key(KeyCode::Esc));
        assert!(app.overlays.is_empty(), "esc closes the sheet from the list");
    }
}
