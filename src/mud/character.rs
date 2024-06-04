use serde::{Deserialize, Serialize};

use super::{items::Inventory, world::Location};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Attribute(i32);

impl Attribute {
    pub fn new(base: i32) -> Self {
        Self(base)
    }

    pub fn value(&self) -> i32 {
        self.0
    }

    pub fn modifier(&self) -> i32 {
        (self.0 - 10) / 2
    }
}

impl Default for Attribute {
    fn default() -> Self {
        Self(10)
    }
}

#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Attributes {
    pub strength: Attribute,
    pub toughness: Attribute,
    pub agility: Attribute,
    pub intelligence: Attribute,
    pub willpower: Attribute,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Character {
    pub name: String,
    pub location: Location,
    pub health: u32,
    pub attributes: Attributes,
    pub inventory: Inventory,
}

impl Character {
    pub fn max_health(&self) -> u32 {
        ((self.attributes.toughness.modifier() * 2) + 8)
            .max(1)
            .try_into()
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use crate::mud::items::ItemStack;

    use super::*;

    #[test]
    fn test_attributes() {
        assert_eq!(Attribute::new(12).modifier(), 1);
        assert_eq!(Attribute::new(8).modifier(), -1);
        assert_eq!(Attribute::new(10).modifier(), 0);
        assert_eq!(Attribute::new(11).modifier(), 0);
        assert_eq!(Attribute::new(13).modifier(), 1);
        assert_eq!(Attribute::new(17).modifier(), 3);

        let mut steve = Character::default();
        for (i, h) in [(2, 1), (8, 6), (10, 8), (14, 12)] {
            steve.attributes.toughness.0 = i;
            assert_eq!(steve.max_health(), h);
        }
    }

    #[test]
    fn test_inventory() {
        let torch = "Torch";
        let sword = "Sword";
        let gold = "Gold Coin";

        let mut inv = Inventory::default();

        inv.add(&torch, 2);

        assert_eq!(
            inv.get("Torch"),
            Some(ItemStack {
                name: torch.to_string(),
                count: 2
            })
            .as_ref()
        );

        assert_eq!(inv.get("Sword"), None);

        inv.add(&gold, 3);
        inv.add(&sword, 1);

        assert_eq!(inv.total_weight(), 6);

        assert!(inv.remove(&torch, 1));
        assert_eq!(
            inv.get("Torch"),
            Some(ItemStack {
                name: torch.to_string(),
                count: 1
            })
            .as_ref()
        );
        assert!(inv.remove(&torch, 1));
        assert_eq!(inv.get("Torch"), None);
        assert_eq!(inv.total_weight(), 4);
    }

    #[test]
    fn save_character() {
        let mut ada = Character::default();

        ada.inventory.add("Fur Armor", 1);
        ada.inventory.add("Battle Axe", 2);
        ada.inventory.add("Gold Coins", 12);

        ada.attributes.strength.0 = 16;
        ada.attributes.toughness.0 = 18;
        ada.attributes.intelligence.0 = 8;

        let ada_serde = serde_yaml::to_string(&ada).unwrap();
        let ada_de: Character = serde_yaml::from_str(&ada_serde).unwrap();

        assert_eq!(ada, ada_de);

        println!("{}", serde_yaml::to_string(&ada_de).unwrap());
    }
}
