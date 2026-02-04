use std::marker::PhantomData;

pub trait ArenaId: Copy {
    fn from_usize(value: usize) -> Self;
    fn into_usize(self) -> usize;
}

#[derive(Debug, Clone)]
pub struct Arena<T, I: ArenaId> {
    items: Vec<T>,
    _marker: PhantomData<I>,
}

impl<T, I: ArenaId> Default for Arena<T, I> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T, I: ArenaId> Arena<T, I> {
    pub fn alloc(&mut self, value: T) -> I {
        let id = I::from_usize(self.items.len());
        self.items.push(value);
        id
    }

    pub fn get(&self, id: I) -> &T {
        &self.items[id.into_usize()]
    }

    pub fn get_mut(&mut self, id: I) -> &mut T {
        &mut self.items[id.into_usize()]
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
