// use std::{collections::HashMap, sync::Arc};

// use indicatif::{ParallelProgressIterator, ProgressStyle};
// use rayon::prelude::*;

// use crate::{Peak, PeakReader, utils::Synced};

// #[derive(Debug)]
// enum Node<T> {
//     Loading,
//     Ready {
//         index: usize,
//         data: Arc<T>,
//         neighbors: HashMap<usize, Arc<T>>,
//     },
// }

// impl<T> Node<T> {
//     fn new(index: usize, data: Arc<T>) -> Self {
//         Self::Ready {
//             index,
//             data,
//             neighbors: HashMap::new(),
//         }
//     }

//     fn get_data(&self) -> Option<Arc<T>> {
//         if let Self::Ready { data, .. } = self {
//             Some(Arc::clone(data))
//         } else {
//             None
//         }
//     }

//     fn get_neighbors(&self) -> Option<&HashMap<usize, Arc<T>>> {
//         if let Self::Ready { neighbors, .. } = self {
//             Some(neighbors)
//         } else {
//             None
//         }
//     }
// }

// pub trait GraphProcessor<T> {
//     fn load(&self, index: usize) -> Option<T>;
//     fn neighbor_indices(&self, index: usize, data: Arc<T>) -> Vec<usize>;
//     fn process(&self, data: &Arc<T>, neighbors: &HashMap<usize, Arc<T>>);
// }

// pub struct Graph<T, P: GraphProcessor<T>> {
//     buffer: Synced<HashMap<usize, Node<T>>>,
//     processor: P,
// }

// impl<T, P: GraphProcessor<T>> Graph<T, P> {
//     pub fn new(processor: P) -> Self {
//         Self {
//             buffer: Synced::default(),
//             processor,
//         }
//     }

//     fn load(&self, index: usize) -> Option<Arc<T>> {
//         loop {
//             match self
//                 .buffer
//                 .with_lock(|b| match b.get(&index) {
//                     Some(opt) => opt.get_data(),
//                     _ => unreachable!("set in previous step"),
//                 })
//                 .unwrap()
//             {
//                 Some(data) => return Some(data),
//                 None => {},
//             }
//             std::thread::sleep(std::time::Duration::from_millis(1));
//         }
//     }

//     fn set(&self, index: usize) -> Option<Arc<T>> {
//         let result = Arc::new(self.processor.load(index)?);
//         _ = self.buffer.with_lock(|b| {
//             b.insert(index, Node::new(index, result.clone()));
//         });
//         Some(result)
//     }

//     fn is_available(&self, index: usize) -> bool {
//         self.buffer
//             .with_lock(|b| match b.get_mut(&index) {
//                 Some(_) => true,
//                 None => {
//                     b.insert(index, Node::Loading);
//                     false
//                 },
//             })
//             .unwrap()
//     }

//     fn get(&self, index: usize) -> Option<Arc<T>> {
//         if self.is_available(index) {
//             self.load(index)
//         } else {
//             self.set(index)
//         }
//     }

//     fn get_neighbors(&self, index: usize) -> Option<HashMap<usize, Arc<T>>> {
//         let data = self.get(index)?;
//         let neighbor_indices =
//             self.processor.neighbor_indices(index, data.clone());
//         let neighbors_todo = self
//             .buffer
//             .with_lock(|b| {
//                 let neighbors = b
//                     .get(&index)
//                     .expect("Ready node")
//                     .get_neighbors()
//                     .expect("Ready node");
//                 let neighbors_todo = neighbor_indices
//                     .iter()
//                     .filter(|i| !neighbors.contains_key(i))
//                     .filter(|i| match b.get(i) {
//                         Some(Node::Ready { .. }) => false,
//                         Some(Node::Loading) => false,
//                         None => {
//                             b.insert(**i, Node::Loading);
//                             true
//                         },
//                     })
//                     .collect::<Vec<_>>();
//                 neighbors_todo
//             })
//             .unwrap();
//         let neighbors_todo = neighbor_indices
//             .into_iter()
//             .map(|i| {
//                 let neighbor = self
//                     .buffer
//                     .with_lock(|b| {
//                         let neighbors = b
//                             .get(&index)
//                             .expect("Ready node")
//                             .get_neighbors()
//                             .expect("Ready node");
//                     })
//                     .unwrap();
//                 (i, neighbor.unwrap())
//             })
//             .collect::<HashMap<_, _>>();
//         Some(neighbors)
//     }

//     pub fn process(&self, index: usize) -> Option<()> {
//         let data = self.get(index)?;
//         let neighbors = self.get_neighbors(index)?;
//         self.processor.process(&data, &neighbors);
//         _ = self.remove(index);
//         Some(())
//     }

//     fn remove(&self, index: usize) -> Option<()> {
//         match self.buffer.with_lock(|b| b.remove(&index)).unwrap() {
//             Some(_) => Some(()),
//             None => None,
//         }
//     }
// }

// struct FramePeaks {
//     index: usize,
//     peaks: Vec<Peak>,
// }

// struct PeakProcessor {
//     peak_loader: PeakReader,
// }

// impl GraphProcessor<FramePeaks> for PeakProcessor {
//     fn load(&self, index: usize) -> Option<FramePeaks> {
//         let peaks = self.peak_loader.get_peaks_from_frame(index).ok()?;
//         Some(FramePeaks { index, peaks })
//     }

//     fn neighbor_indices(
//         &self,
//         index: usize,
//         data: Arc<FramePeaks>,
//     ) -> Vec<usize> {
//         todo!("Define neighbor logic");
//         let mut neighbors = Vec::new();
//         if index > 0 {
//             neighbors.push(index - 1);
//         }
//         neighbors.push(index + 1);
//         neighbors
//     }

//     fn process(
//         &self,
//         data: &Arc<FramePeaks>,
//         neighbors: &HashMap<usize, Arc<FramePeaks>>,
//     ) {
//         todo!("Implement peak processing with neighbors");
//     }
// }

// pub fn run(peak_reader: PeakReader) {
//     let len = peak_reader.frame_count();
//     let processor = PeakProcessor {
//         peak_loader: peak_reader,
//     };
//     let buffer = Graph::new(processor);
//     // todo!("Set atomuic usize for index order");
//     (0..len)
//         .into_par_iter()
//         .progress_with_style(
//             ProgressStyle::default_bar()
//                 .template(" [{elapsed_precise}] {bar} {pos:>7}/{len:7} ({eta}, {per_sec} frames/s)")
//                 .expect("Failed to set progress style")
//         )
//         .for_each(|index| {
//             buffer.process(index);
//         });
// }

use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Condvar, Mutex},
};

// ------------------ Node state ------------------

#[derive(Debug)]
enum NodeState<T> {
    Loading,
    Ready(Arc<T>),
}

struct Node<T> {
    state: Mutex<NodeState<T>>,
    ready: Condvar,
    ref_count: AtomicUsize, // counts threads and neighbors
}

impl<T> Node<T> {
    fn new_loading() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(NodeState::Loading),
            ready: Condvar::new(),
            ref_count: AtomicUsize::new(1), // initial ref for the loader
        })
    }

    fn set_ready(&self, data: Arc<T>) {
        let mut state = self.state.lock().unwrap();
        *state = NodeState::Ready(data);
        self.ready.notify_all();
    }

    fn wait_ready(&self) -> Arc<T> {
        let mut state = self.state.lock().unwrap();
        loop {
            match &*state {
                NodeState::Ready(data) => return Arc::clone(data),
                NodeState::Loading => {
                    state = self.ready.wait(state).unwrap();
                },
            }
        }
    }

    fn increment(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    fn decrement(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Release) - 1
    }
}

// ------------------ Trait for user ------------------

pub trait GraphProcessor<T>: Sync + Send {
    fn load(&self, index: usize) -> Option<T>;
    fn neighbor_indices(&self, index: usize, data: Arc<T>) -> Vec<usize>;
    fn process(&self, data: &Arc<T>, neighbors: &HashMap<usize, Arc<T>>);
}

// ------------------ Graph ------------------

pub struct Graph<T, P: GraphProcessor<T>> {
    buffer: Mutex<HashMap<usize, Arc<Node<T>>>>,
    processor: Arc<P>,
}

impl<T: Send + Sync + 'static, P: GraphProcessor<T>> Graph<T, P> {
    pub fn new(processor: P) -> Self {
        Self {
            buffer: Mutex::new(HashMap::new()),
            processor: Arc::new(processor),
        }
    }

    fn get(&self, index: usize) -> Option<Arc<T>> {
        // Fast path: node already exists
        if let Some(node) = self.buffer.lock().unwrap().get(&index) {
            node.increment();
            return Some(node.wait_ready());
        }

        // Create new loading node
        let node = Node::new_loading();
        let mut buffer = self.buffer.lock().unwrap();
        // Another thread might have inserted in the meantime
        let node = match buffer.entry(index) {
            std::collections::hash_map::Entry::Occupied(e) => {
                let existing: &Arc<Node<T>> = e.get();
                existing.increment();
                existing.clone()
            },
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(node.clone());
                node
            },
        };
        drop(buffer);

        // Load data only if we were the loader
        let state = node.state.lock().unwrap();
        if let NodeState::Loading = &*state {
            drop(state); // release lock before loading
            let data = Arc::new(self.processor.load(index)?);
            node.set_ready(data.clone());
            Some(data)
        } else {
            Some(node.wait_ready())
        }
    }

    fn get_neighbors(
        &self,
        index: usize,
        data: Arc<T>,
    ) -> HashMap<usize, Arc<T>> {
        let mut neighbors = HashMap::new();
        for n_idx in self.processor.neighbor_indices(index, data.clone()) {
            if let Some(n_data) = self.get(n_idx) {
                neighbors.insert(n_idx, n_data);
            }
        }
        neighbors
    }

    pub fn process(&self, index: usize) -> Option<()> {
        let data = self.get(index)?;
        let neighbors = self.get_neighbors(index, data.clone());
        self.processor.process(&data, &neighbors);

        // Decrement self
        #[allow(clippy::collapsible_if)]
        if let Some(node) = self.buffer.lock().unwrap().get(&index) {
            if node.decrement() == 0 {
                self.buffer.lock().unwrap().remove(&index);
            }
        }

        // Decrement neighbors
        #[allow(clippy::collapsible_if)]
        for (n_idx, _) in neighbors {
            if let Some(node) = self.buffer.lock().unwrap().get(&n_idx) {
                if node.decrement() == 0 {
                    self.buffer.lock().unwrap().remove(&n_idx);
                }
            }
        }

        Some(())
    }

    // Optional: process all indices in parallel
    pub fn process_all<I: IntoParallelIterator<Item = usize>>(
        &self,
        indices: I,
    ) {
        indices.into_par_iter().for_each(|i| {
            self.process(i);
        });
    }
}
