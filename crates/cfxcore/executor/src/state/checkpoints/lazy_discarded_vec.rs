use std::{collections::HashMap, hash::Hash};

use super::CheckpointEntry;

pub trait CheckpointLayerTrait {
    type Key: Eq + Hash;
    type Value;
    type ExtInfo;

    fn get_additional_info(&self) -> Self::ExtInfo;

    fn as_hash_map(
        &mut self,
    ) -> &mut HashMap<Self::Key, CheckpointEntry<Self::Value>>;

    fn update(
        self, cache: &mut HashMap<Self::Key, Self::Value>, self_id: usize,
    );

    fn insert_on_absent(
        &mut self, key: Self::Key, value: CheckpointEntry<Self::Value>,
    ) -> bool {
        use std::collections::hash_map::Entry::*;
        match self.as_hash_map().entry(key) {
            Occupied(_) => false,
            Vacant(e) => {
                e.insert(value);
                true
            }
        }
    }
}

#[derive(Debug)]
pub struct LazyDiscardedVec<T: CheckpointLayerTrait> {
    inner_vec: Vec<T>,
    undiscard_indices: Vec<usize>,
}

impl<T: CheckpointLayerTrait> Default for LazyDiscardedVec<T> {
    fn default() -> Self {
        Self {
            inner_vec: Default::default(),
            undiscard_indices: Default::default(),
        }
    }
}

impl<T: CheckpointLayerTrait> LazyDiscardedVec<T> {

    #[inline]
    fn total_len(&self) -> usize { self.inner_vec.len() }

    #[inline]
    fn undiscarded_len(&self) -> usize { self.undiscard_indices.len() }

    pub fn is_empty(&self) -> bool { self.undiscarded_len() == 0 }

    pub fn add_element(&mut self, new_element: T) -> usize {
        self.undiscard_indices.push(self.total_len());
        self.inner_vec.push(new_element);
        self.undiscard_indices.len() - 1
    }

    pub fn clear_elements(&mut self) {
        self.inner_vec = Vec::new();
        self.undiscard_indices = Vec::new();
    }

    pub fn discard_element(&mut self, clear_empty: bool) -> Option<usize> {
        let undiscarded_len = self.undiscarded_len();

        if undiscarded_len == 0 {
            return None
        }

        let current_discard_index = self.undiscard_indices.pop().unwrap();

        if undiscarded_len == 1 && clear_empty {
            self.clear_elements();
        }
        
        Some(current_discard_index)
    }

    pub fn revert_element(
        &mut self, cache: &mut HashMap<T::Key, T::Value>,
    ) -> Option<T::ExtInfo> {
        let current_discard_index = self.discard_element(false)?;
        let last_element_id = self.total_len() - 1;
        assert!(current_discard_index <= last_element_id);
        let revert_elements = self.inner_vec.split_off(current_discard_index);
        let additional_info = revert_elements[0].get_additional_info();
        for (id_from_last, one_revert_element) in
            revert_elements.into_iter().rev().enumerate()
        {
            one_revert_element.update(cache, last_element_id - id_from_last);
        }
        Some(additional_info)
    }

    pub fn get_info_of_last_element(&self) -> Option<T::ExtInfo> {
        if self.undiscarded_len() == 0 {
            assert_eq!(self.total_len(), 0);
            return None;
        }

        Some(self.inner_vec.last().unwrap().get_additional_info())
    }

    #[cfg(test)]
    pub fn get_info_of_all_elements(&self) -> Vec<T::ExtInfo> {
        self.inner_vec
            .iter()
            .map(|element| element.get_additional_info())
            .collect()
    }

    pub fn notify_last_element(
        &mut self, key: T::Key, value: CheckpointEntry<T::Value>,
    ) -> Option<Option<usize>> {
        if self.undiscarded_len() == 0 {
            assert_eq!(self.total_len(), 0);
            return None;
        }

        let last_element = self.inner_vec.last_mut().unwrap();
        let updated = last_element.insert_on_absent(key, value);
        if updated {
            Some(Some(self.total_len() - 1))
        } else {
            Some(None)
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize { self.undiscarded_len() }

    #[cfg(test)]
    pub fn elements_from_index(
        &self, undiscard_element_index: usize,
    ) -> impl Iterator<Item = (&T, usize)> {
        let mut element_index = self.undiscard_indices.len();
        if undiscard_element_index < self.undiscarded_len() {
            for _ in
                (undiscard_element_index..self.undiscarded_len()).rev()
            {
                element_index = self.undiscard_indices[element_index - 1];
            }
        }
        self.inner_vec
            .iter()
            .skip(element_index)
            .zip(element_index..self.total_len())
    }
}
