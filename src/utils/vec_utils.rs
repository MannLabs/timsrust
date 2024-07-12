pub fn argsort<T: Ord>(vec: &Vec<T>) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..vec.len()).collect();
    indices.sort_by_key(|&i| &vec[i]);
    indices
}

pub fn group_and_sum<T: Ord + Copy, U: std::ops::Add<Output = U> + Copy>(
    groups: Vec<T>,
    values: Vec<U>,
) -> (Vec<T>, Vec<U>) {
    if groups.len() == 0 {
        return (vec![], vec![]);
    }
    let order: Vec<usize> = argsort(&groups);
    let mut new_groups: Vec<T> = Vec::with_capacity(order.len());
    let mut new_values: Vec<U> = Vec::with_capacity(order.len());
    let mut current_group: T = groups[order[0]];
    let mut current_value: U = values[order[0]];
    for &index in &order[1..] {
        let group: T = groups[index];
        let value: U = values[index];
        if group != current_group {
            new_groups.push(current_group);
            new_values.push(current_value);
            current_group = group;
            current_value = value;
        } else {
            current_value = current_value + value;
        };
    }
    new_groups.push(current_group);
    new_values.push(current_value);
    (new_groups, new_values)
}

pub fn find_sparse_local_maxima_mask(
    indices: &Vec<u32>,
    values: &Vec<u64>,
    window: u32,
) -> Vec<bool> {
    let mut local_maxima: Vec<bool> = vec![true; indices.len()];
    for (index, sparse_index) in indices.iter().enumerate() {
        let current_intensity: u64 = values[index];
        for (_next_index, next_sparse_index) in
            indices[index + 1..].iter().enumerate()
        {
            let next_index: usize = _next_index + index + 1;
            let next_value: u64 = values[next_index];
            if (next_sparse_index - sparse_index) <= window {
                if current_intensity < next_value {
                    local_maxima[index] = false
                } else {
                    local_maxima[next_index] = false
                }
            } else {
                break;
            }
        }
    }
    local_maxima
}

pub fn filter_with_mask<T: Copy>(vec: &Vec<T>, mask: &Vec<bool>) -> Vec<T> {
    (0..vec.len())
        .filter(|&x| mask[x])
        .map(|x| vec[x])
        .collect()
}
