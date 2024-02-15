use std::collections::HashSet;
use std::iter::Peekable;
use std::ops::Index;

use crate::common::Date;
use crate::lex::LexItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseKeyword {
    Abandoned,
    Binding,
    ClientHostname,
    Cltt,
    Ends,
    Next,
    Hardware,
    Hostname,
    Rewind,
    Set,
    Starts,
    Uid,
}

impl LeaseKeyword {
    pub fn to_string(&self) -> String {
        match self {
            &Self::Abandoned => "abandoned".to_owned(),
            &Self::Binding => "binding".to_owned(),
            &Self::ClientHostname => "client-hostname".to_owned(),
            &Self::Cltt => "cltt".to_owned(),
            &Self::Ends => "ends".to_owned(),
            &Self::Hardware => "hardware".to_owned(),
            &Self::Hostname => "hostname".to_owned(),
            &Self::Next => "next".to_owned(),
            &Self::Rewind => "rewind".to_owned(),
            &Self::Set => "set".to_owned(),
            &Self::Starts => "starts".to_owned(),
            &Self::Uid => "uid".to_owned(),
        }
    }

    pub fn from(s: &str) -> Result<Self, String> {
        match s {
            "abandoned" => Ok(Self::Abandoned),
            "binding" => Ok(Self::Binding),
            "client-hostname" => Ok(Self::ClientHostname),
            "cltt" => Ok(Self::Cltt),
            "ends" => Ok(Self::Ends),
            "hardware" => Ok(Self::Hardware),
            "hostname" => Ok(Self::Hostname),
            "next" => Ok(Self::Next),
            "rewind" => Ok(Self::Rewind),
            "starts" => Ok(Self::Starts),
            "uid" => Ok(Self::Uid),
            _ => Err(format!("'{}' is not a recognized lease option", s)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseDates {
    pub starts: Option<Date>,
    pub ends: Option<Date>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hardware {
    pub h_type: String,
    pub mac: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeasesField {
    ClientHostname,
    Hostname,
    LeasedIP,
    MAC,
}

impl LeasesField {
    fn value_getter(&self) -> Box<dyn Fn(&Lease) -> Option<String>> {
        match &self {
            LeasesField::ClientHostname => {
                Box::new(|l: &Lease| -> Option<String> { l.client_hostname.clone() })
            }
            LeasesField::Hostname => Box::new(|l: &Lease| -> Option<String> { l.hostname.clone() }),
            LeasesField::LeasedIP => Box::new(|l: &Lease| -> Option<String> { Some(l.ip.clone()) }),
            LeasesField::MAC => Box::new(|l: &Lease| -> Option<String> {
                match &l.hardware {
                    Some(h) => Some(h.mac.clone()),
                    None => None,
                }
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leases(Vec<Lease>);

impl Index<usize> for Leases {
    type Output = Lease;

    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

pub trait LeasesMethods {
    fn all(&self) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn active_by<S: AsRef<str>>(
        &self,
        field_name: LeasesField,
        value: S,
        active_at: Date,
    ) -> Option<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_leased<S: AsRef<str>>(&self, ip: S) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_leased_all<S: AsRef<str>>(&self, ip: S) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_mac<S: AsRef<str>>(&self, mac: S) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_mac_all<S: AsRef<str>>(&self, mac: S) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn active_by_hostname<S: AsRef<str>>(&self, hostname: S, active_at: Date) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease>;

    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn active_by_client_hostname<S: AsRef<str>>(
        &self,
        hostname: S,
        active_at: Date,
    ) -> Option<Lease>;
    #[deprecated(since = "0.4.3", note = "any filtering logic should be done by user")]
    fn by_client_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease>;

    fn new() -> Leases;
    fn push(&mut self, l: Lease);
    fn hostnames(&self) -> HashSet<String>;
    fn client_hostnames(&self) -> HashSet<String>;
}

impl LeasesMethods for Leases {
    fn all(&self) -> Vec<Lease> {
        self.0.clone()
    }

    /// Returns a lease by some field and it's value if it exists.
    ///
    /// The lease has to be active:
    ///
    /// - `active_at` is between it's `starts` and `ends` datetime
    /// - is not `abandoned`
    /// - no active leases that match the field value exist after it
    fn active_by<S: AsRef<str>>(
        &self,
        field: LeasesField,
        value: S,
        active_at: Date,
    ) -> Option<Lease> {
        let expected_val = value.as_ref();
        let get_val = field.value_getter();

        let mut ls = self.0.clone();
        ls.reverse();

        for l in ls {
            if l.is_active_at(active_at) && !l.abandoned {
                let val = get_val(&l);
                if val.is_some() && val.unwrap() == expected_val {
                    return Some(l);
                }
            }
        }

        None
    }

    fn by_leased<S: AsRef<str>>(&self, ip: S) -> Option<Lease> {
        let mut ls = self.0.clone();
        ls.reverse();

        for l in ls {
            if l.ip == ip.as_ref() {
                return Some(l);
            }
        }

        None
    }

    fn by_leased_all<S: AsRef<str>>(&self, ip: S) -> Vec<Lease> {
        let mut result = Vec::new();
        let ls = self.0.clone();

        for l in ls {
            if l.ip == ip.as_ref() {
                result.push(l);
            }
        }

        return result;
    }

    fn by_mac<S: AsRef<str>>(&self, mac: S) -> Option<Lease> {
        let mut ls = self.0.clone();
        ls.reverse();

        for l in ls {
            let hw = l.hardware.as_ref();
            if hw.is_some() && hw.unwrap().mac == mac.as_ref() {
                return Some(l);
            }
        }

        None
    }

    fn by_mac_all<S: AsRef<str>>(&self, mac: S) -> Vec<Lease> {
        let mut result = Vec::new();
        let ls = self.0.clone();

        for l in ls {
            let hw = l.hardware.as_ref();
            if hw.is_some() && hw.unwrap().mac == mac.as_ref() {
                result.push(l);
            }
        }

        return result;
    }

    fn active_by_hostname<S: AsRef<str>>(&self, hostname: S, active_at: Date) -> Option<Lease> {
        #[allow(deprecated)]
        self.active_by(LeasesField::Hostname, hostname, active_at)
    }

    fn by_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease> {
        let mut res = Vec::new();
        let ls = self.0.clone();
        let hn_s = hostname.as_ref();

        for l in ls {
            let hn = l.hostname.as_ref();
            if hn.is_some() && hn.unwrap() == hn_s {
                res.push(l);
            }
        }

        res
    }

    fn active_by_client_hostname<S: AsRef<str>>(
        &self,
        hostname: S,
        active_at: Date,
    ) -> Option<Lease> {
        #[allow(deprecated)]
        self.active_by(LeasesField::ClientHostname, hostname, active_at)
    }

    fn by_client_hostname_all<S: AsRef<str>>(&self, hostname: S) -> Vec<Lease> {
        let mut res = Vec::new();
        let ls = self.0.clone();
        let hn_s = hostname.as_ref();

        for l in ls {
            let hn = l.client_hostname.as_ref();
            if hn.is_some() && hn.unwrap() == hn_s {
                res.push(l);
            }
        }

        res
    }

    fn new() -> Leases {
        Leases(Vec::new())
    }

    fn push(&mut self, l: Lease) {
        self.0.push(l);
    }

    fn hostnames(&self) -> HashSet<String> {
        let mut res = HashSet::new();
        let ls = self.0.clone();

        for l in ls {
            if l.hostname.is_some() {
                res.insert(l.hostname.unwrap());
            }
        }

        return res;
    }

    fn client_hostnames(&self) -> HashSet<String> {
        let mut res = HashSet::new();
        let ls = self.0.clone();

        for l in ls {
            if l.client_hostname.is_some() {
                res.insert(l.client_hostname.unwrap());
            }
        }

        return res;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub ip: String,
    pub dates: LeaseDates,
    pub hardware: Option<Hardware>,
    pub uid: Option<String>,
    pub client_hostname: Option<String>,
    pub hostname: Option<String>,
    pub abandoned: bool,
    pub cltt: Option<Date>,
    /// Binding state.
    /// When server is not configured to use failover protocol,
    /// the binding state will either be active or free.
    pub binding: Option<String>,
    /// Next binding state, indicates state the lease
    /// will move to when current binding expires.
    pub next_binding: Option<String>,
    /// Rewind binding state
    pub rewind_binding: Option<String>,
}

impl Lease {
    pub fn new() -> Lease {
        Lease {
            ip: "localhost".to_owned(),
            dates: LeaseDates {
                starts: None,
                ends: None,
            },
            hardware: None,
            uid: None,
            cltt: None,
            client_hostname: None,
            hostname: None,
            abandoned: false,
            binding: None,
            next_binding: None,
            rewind_binding: None,
        }
    }

    pub fn is_active_at(&self, when: Date) -> bool {
        if self.dates.starts.is_some() && self.dates.starts.unwrap() > when {
            return false;
        }

        if self.dates.ends.is_some() && self.dates.ends.unwrap() < when {
            return false;
        }

        return true;
    }
}

pub fn parse_lease<'l, T: Iterator<Item = &'l LexItem>>(
    lease: &mut Lease,
    iter: &mut Peekable<T>,
) -> Result<(), String> {
    while let Some(&nc) = iter.peek() {
        match nc {
            LexItem::Opt(LeaseKeyword::Starts) => {
                iter.next();
                let weekday = iter
                    .peek()
                    .expect("Weekday for start date expected")
                    .to_string();
                iter.next();
                let date = iter
                    .peek()
                    .expect("Date for start date expected")
                    .to_string();
                iter.next();
                let time = iter
                    .peek()
                    .expect("Time for start date expected")
                    .to_string();
                iter.next();

                let tz = iter
                    .peek()
                    .expect("Timezone or semicolon expected")
                    .to_string();
                if tz != LexItem::Endl.to_string() {
                    iter.next();
                    match iter.peek().expect("Semicolon expected") {
                        LexItem::Endl => (),
                        s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                    }
                }

                lease.dates.starts.replace(Date::from(weekday, date, time)?);
            }
            LexItem::Opt(LeaseKeyword::Ends) => {
                iter.next();
                let weekday = iter
                    .peek()
                    .expect("Weekday for end date expected")
                    .to_string();
                iter.next();
                let date = iter.peek().expect("Date for end date expected").to_string();
                iter.next();
                let time = iter.peek().expect("Time for end date expected").to_string();
                iter.next();
                let tz = iter
                    .peek()
                    .expect("Timezone or semicolon expected")
                    .to_string();

                if tz != LexItem::Endl.to_string() {
                    iter.next();
                    match iter.peek().expect("Semicolon expected") {
                        LexItem::Endl => (),
                        s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                    }
                }

                lease.dates.ends.replace(Date::from(weekday, date, time)?);
            }
            LexItem::Opt(LeaseKeyword::Hardware) => {
                iter.next();
                let h_type = iter.peek().expect("Hardware type expected").to_string();
                iter.next();
                let mac = iter.peek().expect("MAC address expected").to_string();
                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }

                lease.hardware.replace(Hardware {
                    h_type: h_type,
                    mac: mac,
                });
            }
            LexItem::Opt(LeaseKeyword::Uid) => {
                iter.next();
                lease
                    .uid
                    .replace(iter.peek().expect("Client identifier expected").to_string());

                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            LexItem::Opt(LeaseKeyword::ClientHostname) => {
                iter.next();
                lease.client_hostname.replace(unquote_hostname(
                    iter.peek().expect("Client hostname expected").to_string(),
                ));

                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            LexItem::Opt(LeaseKeyword::Hostname) => {
                iter.next();
                lease.hostname.replace(unquote_hostname(
                    iter.peek().expect("Hostname expected").to_string(),
                ));

                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            LexItem::Opt(LeaseKeyword::Abandoned) => {
                lease.abandoned = true;
                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            LexItem::Opt(LeaseKeyword::Binding) => {
                iter.next();
                
                let _ = iter.peek().expect("Binding state expected").to_string();
                iter.next();

                lease.binding.replace(
                    iter.peek()
                        .expect("Binding identifier expected")
                        .to_string(),
                );

                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            LexItem::Opt(LeaseKeyword::Next) => {
                iter.next();
                
                let _ = iter.peek().expect("Next binding state expected").to_string();
                iter.next();
                
                let _ = iter.peek().expect("Next binding state expected").to_string();
                iter.next();
                
                lease.next_binding.replace(
                    iter.peek()
                        .expect("Next binding state identifier expected")
                        .to_string(),
                );

                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            LexItem::Opt(LeaseKeyword::Rewind) => {
                iter.next();
                
                let _ = iter.peek().expect("Rewind binding state expected").to_string();
                iter.next();
                
                let _ = iter.peek().expect("Rewind binding state expected").to_string();
                iter.next();

                lease.rewind_binding.replace(
                    iter.peek()
                        .expect("Next binding state identifier expected")
                        .to_string(),
                );

                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            // Cltt option is not really exploited at the moment
            LexItem::Opt(LeaseKeyword::Cltt) => {
                iter.next();
                let weekday = iter
                    .peek()
                    .expect("Weekday for cltt date expected")
                    .to_string();
                iter.next();
                let date = iter
                    .peek()
                    .expect("Date for cltt date expected")
                    .to_string();
                iter.next();
                let time = iter
                    .peek()
                    .expect("Time for cltt date expected")
                    .to_string();
                iter.next();

                let tz = iter
                    .peek()
                    .expect("Timezone or semicolon expected")
                    .to_string();
                if tz != LexItem::Endl.to_string() {
                    iter.next();
                    match iter.peek().expect("Semicolon expected") {
                        LexItem::Endl => (),
                        s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                    }
                }

                lease.cltt.replace(Date::from(weekday, date, time)?);
            }
            // Set option is not really exploited at the moment
            LexItem::Opt(LeaseKeyword::Set) => {
                iter.next();
                iter.next();
                match iter.peek().expect("Semicolon expected") {
                    LexItem::Endl => (),
                    s => return Err(format!("Expected semicolon, found {}", s.to_string())),
                }
            }
            LexItem::Paren('}') => {
                return Ok(());
            }
            _ => {
                return Err(format!(
                    "Unexpected option '{}'",
                    iter.peek().unwrap().to_string()
                ));
            }
        }
        iter.next();
    }

    Ok(())
}

fn unquote_hostname(hn: String) -> String {
    hn.replace("\"", "")
}
