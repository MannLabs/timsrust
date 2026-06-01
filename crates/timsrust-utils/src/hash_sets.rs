type FiniteSetIndex = u32;

#[derive(Debug)]
pub struct FiniteSet {
    active: Vec<FiniteSetIndex>,
    pos: Vec<FiniteSetIndex>,
    capacity: FiniteSetIndex,
}

impl FiniteSet {
    pub fn with_capacity(n: FiniteSetIndex) -> Self {
        Self {
            active: Vec::with_capacity(n as usize),
            pos: vec![0; n as usize],
            capacity: n,
        }
    }

    pub fn is_valid(&self, i: FiniteSetIndex) -> bool {
        i < self.capacity
    }

    pub fn insert(&mut self, i: FiniteSetIndex) -> bool {
        if !self.contains(i) && self.is_valid(i) {
            self.pos[i as usize] = self.active.len() as FiniteSetIndex;
            self.active.push(i);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, i: FiniteSetIndex) -> bool {
        if self.contains(i) {
            let last = self.active.pop().expect("Active set is empty");
            if last != i {
                let p = self.pos[i as usize];
                self.active[p as usize] = last;
                self.pos[last as usize] = p;
            }
            true
        } else {
            false
        }
    }

    pub fn contains(&self, i: FiniteSetIndex) -> bool {
        match self.is_valid(i) {
            true => {
                let p = self.pos[i as usize];
                (p as usize) < self.active.len()
                    && (self.active[p as usize] == i)
            },
            false => false,
        }
    }

    pub fn len(&self) -> usize {
        self.active.len()
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FiniteSetIndex> + '_ {
        self.active.iter()
    }

    pub fn clear(&mut self) {
        self.active.clear();
    }
}

#[derive(Debug)]
pub struct FiniteHashMap<V> {
    keys: FiniteSet,
    values: Vec<Option<V>>,
}

impl<V> FiniteHashMap<V> {
    pub fn capacity(&self) -> FiniteSetIndex {
        self.keys.capacity
    }

    pub fn with_capacity(n: FiniteSetIndex) -> Self {
        let mut values = Vec::with_capacity(n as usize);
        values.resize_with(n as usize, || None);
        Self {
            keys: FiniteSet::with_capacity(n),
            values,
        }
    }

    pub fn insert(&mut self, i: FiniteSetIndex, v: V) -> bool {
        if self.keys.insert(i) {
            self.values[i as usize] = Some(v);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, i: FiniteSetIndex) -> bool {
        if self.keys.remove(i) {
            self.values[i as usize] = None;
            true
        } else {
            false
        }
    }

    pub fn get(&self, i: FiniteSetIndex) -> Option<&V> {
        self.values[i as usize].as_ref()
    }

    pub fn contains_key(&self, i: FiniteSetIndex) -> bool {
        self.keys.contains(i)
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn iter_unstable(
        &self,
    ) -> impl Iterator<Item = (FiniteSetIndex, &V)> + '_ {
        self.keys
            .iter()
            .filter_map(move |&i| self.get(i).map(|v| (i, v)))
    }

    pub fn iter_sorted(
        &self,
    ) -> impl Iterator<Item = (FiniteSetIndex, &V)> + '_ {
        let mut keys = self.keys.active.clone();
        keys.sort_unstable();
        keys.into_iter()
            .filter_map(move |i| self.get(i).map(|v| (i, v)))
    }

    pub fn clear(&mut self) {
        for key in self.keys.iter() {
            self.values[*key as usize] = None;
        }
        self.keys.clear();
    }
}

#[derive(Debug, Default)]
pub struct SemiDenseMap<K, V>
where
    K: Copy + Default + Eq + TryInto<usize> + TryFrom<usize>,
{
    positions: Vec<K>,
    active: Vec<K>,
    values: Vec<V>,
}

impl<K, V> SemiDenseMap<K, V>
where
    K: Copy + Default + Eq + TryInto<usize> + TryFrom<usize>,
{
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            active: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            positions: vec![K::default(); n],
            active: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn with_capacity_and_density(n: usize, d: usize) -> Self {
        Self {
            positions: vec![K::default(); n],
            active: Vec::with_capacity(d),
            values: Vec::with_capacity(d),
        }
    }

    fn get_position(&self, k: K) -> Option<usize> {
        let ku = k.try_into().ok()?;
        let position = *self.positions.get(ku)?;
        let p = position.try_into().ok()?;
        Some(p)
    }

    fn position_equals(&self, k: K, position: usize) -> bool {
        match self.active.get(position) {
            Some(&key) => key == k,
            None => false,
        }
    }

    pub fn contains(&self, k: K) -> bool {
        match self.get_position(k) {
            Some(position) => self.position_equals(k, position),
            None => false,
        }
    }

    pub fn remove(&mut self, k: K) -> Option<V> {
        let position = self.get_position(k)?;
        if self.position_equals(k, position) {
            let last_key = self.active.pop().expect("Active set is empty");
            if last_key != k {
                let last_value = self.values.pop()?;
                self.active[position] = last_key;
                let removed_value =
                    std::mem::replace(&mut self.values[position], last_value);
                let lku = last_key.try_into().ok()?;
                self.positions[lku] = position.try_into().ok()?;
                Some(removed_value)
            } else {
                self.values.pop()
            }
        } else {
            None
        }
    }

    pub fn get(&self, k: K) -> Option<&V> {
        let position = self.get_position(k)?;
        if self.position_equals(k, position) {
            Some(&self.values[position])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, k: K) -> Option<&mut V> {
        let position = self.get_position(k)?;
        if self.position_equals(k, position) {
            Some(&mut self.values[position])
        } else {
            None
        }
    }

    fn get_position_or_extend(
        &mut self,
        k: K,
    ) -> Result<usize, SemiDenseMapError> {
        let ku = k
            .try_into()
            .map_err(|_| SemiDenseMapError::CapacityExceeded)?;
        if self.positions.len() <= ku {
            let new_k = K::try_from(self.active.len())
                .map_err(|_| SemiDenseMapError::CapacityExceeded)?;
            // TODO be smarter about resizing, e.g. double the size instead of just extending to ku
            self.positions.resize(ku + 1, new_k);
        }
        let position = self.positions[ku];
        let p = position
            .try_into()
            .map_err(|_| SemiDenseMapError::CapacityExceeded)?;
        Ok(p)
    }

    fn insert_at_position(
        &mut self,
        k: K,
        v: V,
        position: usize,
    ) -> Result<(), SemiDenseMapError> {
        if self.position_equals(k, position) {
            Err(SemiDenseMapError::AlreadyExists)
        } else {
            if self.active.len() != position {
                let ku = k
                    .try_into()
                    .map_err(|_| SemiDenseMapError::CapacityExceeded)?;
                self.positions[ku] = K::try_from(self.active.len())
                    .map_err(|_| SemiDenseMapError::CapacityExceeded)?;
            }
            self.values.push(v);
            self.active.push(k);
            Ok(())
        }
    }

    pub fn insert(&mut self, k: K, v: V) -> Result<(), SemiDenseMapError> {
        let position = self.get_position_or_extend(k)?;
        self.insert_at_position(k, v, position)
    }

    pub fn get_mut_or_default(
        &mut self,
        k: K,
    ) -> Result<&mut V, SemiDenseMapError>
    where
        V: Default,
    {
        let position = self.get_position_or_extend(k)?;
        match self.insert_at_position(k, V::default(), position) {
            Err(SemiDenseMapError::CapacityExceeded) => {
                Err(SemiDenseMapError::CapacityExceeded)
            },
            _ => {
                let final_position = self
                    .get_position(k)
                    .ok_or(SemiDenseMapError::CapacityExceeded)?;
                Ok(&mut self.values[final_position])
            },
        }
    }

    pub fn len(&self) -> usize {
        self.active.len()
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn clear(&mut self) {
        self.active.clear();
        self.values.clear();
    }

    pub fn iter_keys_unstable(&self) -> impl Iterator<Item = &K> + '_ {
        self.active.iter()
    }

    pub fn iter_values_unstable(&self) -> impl Iterator<Item = &V> + '_ {
        self.values.iter()
    }

    pub fn iter_unstable(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        self.active.iter().zip(self.values.iter())
    }

    pub fn iter_sorted(&self) -> impl Iterator<Item = (&K, &V)> + '_
    where
        K: Ord,
    {
        let mut indices = (0..self.active.len()).collect::<Vec<_>>();
        indices.sort_unstable_by_key(|&k| self.active[k]);
        indices
            .into_iter()
            .map(move |k| (&self.active[k], &self.values[k]))
    }

    pub fn iter_mut_values_unstable(
        &mut self,
    ) -> impl Iterator<Item = &mut V> + '_ {
        self.values.iter_mut()
    }

    pub fn iter_mut_unstable(
        &mut self,
    ) -> impl Iterator<Item = (&mut K, &mut V)> + '_ {
        self.active.iter_mut().zip(self.values.iter_mut())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SemiDenseMapError {
    CapacityExceeded,
    AlreadyExists,
}

impl std::fmt::Display for SemiDenseMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemiDenseMapError::CapacityExceeded => {
                write!(f, "Semi-dense map capacity exceeded")
            },
            SemiDenseMapError::AlreadyExists => {
                write!(f, "Key already exists in semi-dense map")
            },
        }
    }
}

impl std::error::Error for SemiDenseMapError {}
