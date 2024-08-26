use cosmwasm_std::{Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Map};
use serde::{de::DeserializeOwned, Serialize};

// QueueMap that implements a double-ended queue using a map and two indexes underneath to allow inserting and removing at certain positions
pub struct QueueMap<T> {
    elements: Map<u64, T>,
    start_index: u64,
    end_index: u64,
}

impl<T> QueueMap<T>
where
    T: Serialize + DeserializeOwned,
{
    pub const fn new(elements_namespace: &'static str) -> Self {
        Self {
            elements: Map::new(elements_namespace),
            start_index: 0,
            end_index: 0,
        }
    }

    fn get_start_index(&self) -> u64 {
        self.start_index
    }

    fn get_end_index(&self) -> u64 {
        self.end_index
    }

    pub fn push_back(&mut self, storage: &mut dyn Storage, value: &T) -> StdResult<()> {
        let mut end_index = self.get_end_index();
        end_index += 1;
        self.elements.save(storage, end_index, value)?;
        self.end_index = end_index;
        Ok(())
    }

    pub fn pop_front(&mut self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        let start_index = self.get_start_index();
        let end_index = self.get_end_index();

        if start_index == end_index {
            return Ok(None);
        }

        let value = self.elements.load(storage, start_index + 1)?;
        self.elements.remove(storage, start_index + 1);
        self.start_index = start_index + 1;

        Ok(Some(value))
    }

    pub fn insert_at(&mut self, storage: &mut dyn Storage, index: u64, value: &T) -> StdResult<()> {
        let len = self.len();
        if index > len {
            return Err(StdError::generic_err("Index out of bounds"));
        }

        let start_index = self.get_start_index();
        let end_index = self.get_end_index();
        let actual_index = start_index + index + 1;

        // Shift elements to make room
        for i in (actual_index..end_index + 2).rev() {
            if let Ok(elem) = self.elements.load(storage, i) {
                self.elements.save(storage, i + 1, &elem)?;
            }
        }

        // Insert the new element
        self.elements.save(storage, actual_index, value)?;
        self.end_index = end_index + 1;

        Ok(())
    }

    pub fn remove_at(&mut self, storage: &mut dyn Storage, index: u64) -> StdResult<T> {
        let len = self.len();
        if index >= len {
            return Err(StdError::generic_err("Index out of bounds"));
        }

        // Special optimization for removing from the front, which is the most common scenario
        // and the most critical (queue can have many elements and the first one can block the queue)
        // This way we make it O(1) instead of O(n) and avoid the possibility to run out of gas to
        // remove the first element
        if index == 0 {
            self.pop_front(storage).map(|v| v.unwrap())
        } else {
            let start_index = self.get_start_index();
            let end_index = self.get_end_index();
            let actual_index = start_index + index + 1;

            // Remove the element
            let value = self.elements.load(storage, actual_index)?;
            self.elements.remove(storage, actual_index);

            // Shift elements to fill the gap
            for i in actual_index + 1..end_index + 1 {
                if let Ok(elem) = self.elements.load(storage, i) {
                    self.elements.save(storage, i - 1, &elem)?;
                    self.elements.remove(storage, i);
                }
            }

            self.end_index = end_index - 1;

            Ok(value)
        }
    }

    pub fn query(
        &self,
        storage: &dyn Storage,
        start: Option<u64>,
        end: Option<u64>,
        order: Order,
    ) -> StdResult<Vec<T>> {
        let start_index = self.get_start_index();
        let end_index = self.get_end_index();
        let queue_len = end_index.saturating_sub(start_index);

        let start = start.unwrap_or(0);
        let end = end.unwrap_or(queue_len);

        if start > end || end > queue_len {
            return Err(StdError::generic_err("Invalid range"));
        }

        let actual_start = start_index + start + 1;
        let actual_end = start_index + end + 1;

        let range_result: StdResult<Vec<T>> = self
            .elements
            .range(
                storage,
                Some(Bound::inclusive(actual_start)),
                Some(Bound::inclusive(actual_end.saturating_sub(1))),
                order,
            )
            .map(|item| item.map(|(_, v)| v))
            .collect();

        range_result
    }

    pub fn len(&self) -> u64 {
        let start_index = self.get_start_index();
        let end_index = self.get_end_index();
        end_index.saturating_sub(start_index)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_push_and_pop() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;
        let mut queue = QueueMap::new("elements");

        queue.push_back(storage, &"first".to_string()).unwrap();
        queue.push_back(storage, &"second".to_string()).unwrap();
        queue.push_back(storage, &"third".to_string()).unwrap();

        assert_eq!(queue.len(), 3);

        assert_eq!(queue.pop_front(storage).unwrap(), Some("first".to_string()));
        assert_eq!(
            queue.pop_front(storage).unwrap(),
            Some("second".to_string())
        );
        assert_eq!(queue.pop_front(storage).unwrap(), Some("third".to_string()));
        assert_eq!(queue.pop_front(storage).unwrap(), None);

        assert!(queue.is_empty());
    }

    #[test]
    fn test_insert_and_remove_at() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;
        let mut queue = QueueMap::new("elements");

        queue.push_back(storage, &"first".to_string()).unwrap();
        queue.push_back(storage, &"third".to_string()).unwrap();
        queue.insert_at(storage, 1, &"second".to_string()).unwrap();

        assert_eq!(queue.len(), 3);

        assert_eq!(queue.remove_at(storage, 1).unwrap(), "second".to_string());
        assert_eq!(queue.len(), 2);

        let items = queue.query(storage, None, None, Order::Ascending).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "first".to_string());
        assert_eq!(items[1], "third".to_string());

        assert_eq!(queue.remove_at(storage, 0).unwrap(), "first".to_string());
        let items = queue.query(storage, None, None, Order::Ascending).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], "third".to_string());
    }

    #[test]
    fn test_query() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;
        let mut queue = QueueMap::new("elements");

        for i in 0..5 {
            queue.push_back(storage, &i.to_string()).unwrap();
        }

        // Test full range
        let items = queue.query(storage, None, None, Order::Ascending).unwrap();
        assert_eq!(items, vec!["0", "1", "2", "3", "4"]);

        // Test partial range
        let items = queue
            .query(storage, Some(1), Some(4), Order::Ascending)
            .unwrap();
        assert_eq!(items, vec!["1", "2", "3"]);

        // Test descending order
        let items = queue.query(storage, None, None, Order::Descending).unwrap();
        assert_eq!(items, vec!["4", "3", "2", "1", "0"]);

        // Test invalid range
        assert!(queue
            .query(storage, Some(3), Some(1), Order::Ascending)
            .is_err());
    }

    #[test]
    fn test_out_of_bounds() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;
        let mut queue = QueueMap::new("elements");

        queue.push_back(storage, &"first".to_string()).unwrap();

        assert!(queue.insert_at(storage, 2, &"third".to_string()).is_err());
        assert!(queue.remove_at(storage, 1).is_err());
    }

    #[test]
    fn test_complex_operations() {
        let mut deps = mock_dependencies();
        let storage = &mut deps.storage;
        let mut queue = QueueMap::new("elements");

        queue.push_back(storage, &"1".to_string()).unwrap();
        queue.push_back(storage, &"2".to_string()).unwrap();
        queue.insert_at(storage, 1, &"1.5".to_string()).unwrap();

        assert_eq!(queue.len(), 3);

        let items = queue.query(storage, None, None, Order::Ascending).unwrap();
        assert_eq!(items, vec!["1", "1.5", "2"]);

        queue.remove_at(storage, 1).unwrap();

        let items = queue.query(storage, None, None, Order::Ascending).unwrap();
        assert_eq!(items, vec!["1", "2"]);

        assert_eq!(queue.pop_front(storage).unwrap(), Some("1".to_string()));

        let items = queue.query(storage, None, None, Order::Ascending).unwrap();
        assert_eq!(items, vec!["2"]);
    }
}
