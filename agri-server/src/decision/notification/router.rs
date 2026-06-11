use chrono::{Local, NaiveTime};

#[derive(Debug, Clone)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub role: Role,
    pub push_token: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Primary,
    Backup,
    Team,
}

#[derive(Debug, Clone)]
pub struct ShiftSlot {
    pub label: &'static str,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub primary: Vec<Contact>,
    pub backup: Vec<Contact>,
}

#[derive(Debug, Clone)]
pub struct RouteResult {
    pub primary: Vec<Contact>,
    pub backup: Vec<Contact>,
}

pub struct ShiftRouter {
    slots: Vec<ShiftSlot>,
}

impl ShiftRouter {
    pub fn new(slots: Vec<ShiftSlot>) -> Self {
        Self { slots }
    }

    pub fn current_on_duty(&self) -> Option<RouteResult> {
        let now = Local::now().time();
        let mut best: Option<&ShiftSlot> = None;
        for slot in &self.slots {
            if slot.start <= slot.end {
                if now >= slot.start && now <= slot.end {
                    best = Some(slot);
                    break;
                }
            } else {
                if now >= slot.start || now <= slot.end {
                    best = Some(slot);
                    break;
                }
            }
        }
        best.map(|s| RouteResult {
            primary: s.primary.clone(),
            backup: s.backup.clone(),
        })
    }

    pub fn is_night(&self) -> bool {
        if let Some(result) = self.current_on_duty() {
            let _ = result;
            return false;
        }
        let now = Local::now().time();
        now < NaiveTime::from_hms_opt(6, 0, 0).unwrap() || now >= NaiveTime::from_hms_opt(22, 0, 0).unwrap()
    }
}
