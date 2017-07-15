use std::collections::HashSet;

#[derive(Hash, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

#[derive(Hash, Copy, Clone, Debug, PartialEq, Eq)]
pub enum UpdateSpec {
    On(Weekday),
    Every(usize),
    Comics(usize),
    Overlap(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Feed {
    pub name: String,
    pub url: String,
    pub updates: HashSet<UpdateSpec>,
}
