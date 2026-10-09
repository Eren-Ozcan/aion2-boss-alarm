//! Boss ids of the in-game boss lists and their names. The ids come from
//! matching on-screen timers against the packet (docs/boss-list-packet.md).

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Field,
    Abyss,
}

impl Group {
    pub fn key(self) -> &'static str {
        match self {
            Group::Field => "field",
            Group::Abyss => "abyss",
        }
    }
}

pub struct BossInfo {
    pub id: u32,
    pub name: &'static str,
    pub group: Group,
}

macro_rules! boss {
    ($id:expr, $name:expr, $g:ident) => {
        BossInfo { id: $id, name: $name, group: Group::$g }
    };
}

pub static BOSSES: &[BossInfo] = &[
    boss!(111001, "Melted Danar", Field),
    boss!(111002, "Black Warrior Aed", Field),
    boss!(111003, "Faithful Rajit", Field),
    boss!(111004, "Berserker Vargor", Field),
    boss!(111005, "Predator Garsan", Field),
    boss!(111006, "Blood Warrior Lannar", Field),
    boss!(111007, "Deceiver Trid", Field),
    boss!(111008, "Blue Wave Kelpina", Field),
    boss!(111009, "High Overseer Nutah", Field),
    boss!(111010, "Advisor Resana", Field),
    boss!(111011, "Special Operations Leader", Field),
    boss!(111012, "Desecrator Newbold", Field),
    boss!(111013, "Specter Archon Axios", Field),
    boss!(111014, "Addicted Hardirun", Field),
    boss!(111015, "Executioner Barthien", Field),
    boss!(111016, "Drakan Battalion Weapon Master", Field),
    boss!(111017, "Veteran Shujakan", Field),
    boss!(111018, "Visionary Karuka", Field),
    boss!(111019, "Dark Shadow Vishwada", Field),
    boss!(111020, "Sharp Shylak", Field),
    boss!(111021, "Immortal Gartua", Field),
    boss!(111022, "High Commander Lagta", Field),
    boss!(111023, "Soul Ruler Kashapa", Field),
    boss!(111024, "Silent Dartan", Field),
    boss!(2001, "Argo, the Spirit King", Abyss),
    boss!(2002, "Watcher Kaira", Abyss),
    boss!(2003, "Guardian Lord Nahma (1)", Abyss),
    boss!(2004, "Guardian Lord Nahma (2)", Abyss),
    boss!(2005, "Guardian Lord Nahma (3)", Abyss),
    boss!(2006, "Executor Tamasa", Abyss),
    boss!(2007, "Executor Argo", Abyss),
    boss!(2008, "Executor Kaira", Abyss),
];

pub fn find(id: u32) -> Option<&'static BossInfo> {
    BOSSES.iter().find(|b| b.id == id)
}
