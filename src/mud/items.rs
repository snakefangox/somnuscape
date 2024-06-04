use serde::{Deserialize, Serialize};

#[derive(Debug, Hash, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub weight: u32,
}

#[derive(Debug, Hash, PartialEq, Eq, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct ItemStack {
    pub name: String,
    pub count: u32,
}

impl ItemStack {
    pub fn new(name: String, count: u32) -> Self {
        Self { name, count }
    }
}

#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Inventory {
    items: Vec<ItemStack>,
}

impl Inventory {
    pub fn get(&self, name: &str) -> Option<&ItemStack> {
        self.items.iter().find(|i| i.name == name)
    }

    fn get_idx(&self, item: &str) -> Option<usize> {
        self.items
            .iter()
            .enumerate()
            .find(|(_, i)| i.name == item)
            .map(|(idx, _)| idx)
    }

    pub fn add(&mut self, item: &str, count: u32) {
        if let Some(idx) = self.get_idx(item) {
            self.items.get_mut(idx).unwrap().count += count;
        } else {
            self.items.push(ItemStack::new(item.to_string(), count));
        }
    }

    pub fn remove(&mut self, item: &str, remove_count: u32) -> bool {
        if remove_count == 0 {
            return false;
        }

        if let Some(idx) = self.get_idx(item) {
            let current_count = self.items[idx].count;
            if current_count < remove_count {
                return false;
            }

            if current_count == remove_count {
                self.items.remove(idx);
            } else {
                self.items.get_mut(idx).unwrap().count -= remove_count;
            }

            true
        } else {
            false
        }
    }

    pub fn total_weight(&mut self) -> u32 {
        self.items.iter().map(|i| i.count).sum()
    }
}