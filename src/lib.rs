
#![allow(dead_code)]
use anyhow::Result;
use anyhow::bail;
use anyhow::anyhow;

pub type TimeseriesUpdateFn<'a, T> = &'a dyn Fn(&mut T);
struct TimeseriesImpl<T> {
    data: Vec<T>,
    capacity: usize,
    len: usize,
    pos: usize,
}
impl<T> TimeseriesImpl<T> {
    pub fn new_with_capacity(capacity: usize) -> Self {
        let mut vec: Vec<std::mem::MaybeUninit<T>> = Vec::with_capacity(capacity * 2);
        unsafe { vec.set_len(capacity * 2); }
        let data = unsafe { std::mem::transmute::<Vec<std::mem::MaybeUninit<T>>, Vec<T>>(vec) };
        Self {
            data,
            capacity,
            len: 0,
            pos: capacity,
        }
    }
    pub fn len(&self) -> usize { self.len }
    pub fn clear(&mut self) {
        self.len = 0;
        self.pos = self.capacity;
    }
    fn pos(&self) -> usize { self.pos }
    fn pos2(&self) -> usize { self.pos + self.len }
    pub fn first(&self) -> Option<&T> { self.get(0) }
    pub fn get(&self, pos: usize) -> Option<&T> {
        if self.len > pos {
            self.data.get(self.pos + pos) 
        }
        else {
            None
        }
    }
    pub fn as_ref_vec(&self) -> Vec<&T> {
        self.data[self.pos()..self.pos2()].iter().collect::<Vec<_>>()
    }
    pub fn as_vec(&self) -> Vec<T> {
        let mut vec: Vec<std::mem::MaybeUninit<T>> = Vec::with_capacity(self.len());
        unsafe { vec.set_len(self.len()); }
        let mut vec = unsafe { std::mem::transmute::<Vec<std::mem::MaybeUninit<T>>, Vec<T>>(vec) };

        unsafe { 
            let src = self.data.as_ptr().add(self.pos);
            let dst = vec.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, self.len);
        }

        vec
    }
    pub fn update(&mut self, value: &T) -> Result<()> {
        if self.len == 0 {
            bail!("Can't update because no entries in time series");
        }
        unsafe {
            let mut dst: *mut T = self.data.get_mut(self.pos).ok_or_else(|| anyhow!("Item at position cannot be found."))?;
            std::ptr::copy_nonoverlapping(value as *const _ as *const T, dst as *mut _ as *mut T, 1);
            dst = dst.add(self.capacity);
            std::ptr::copy_nonoverlapping(value as *const _ as *const T, dst as *mut _ as *mut T, 1);
        }
        Ok(())
    }
    pub fn update_with(&mut self, fun: TimeseriesUpdateFn<T>) -> Result<()> {
        if self.len == 0 {
            bail!("Can't update because no entries in time series");
        }
        for pos in [self.pos(), self.pos2()] {
            let orig = self.data.get_mut(pos).ok_or_else(|| anyhow!("Item at position cannot be found."))?;
            fun(orig);
        }
        Ok(())
    }
    pub fn push(&mut self, value: T) -> Result<()> {
        if self.len < self.capacity {
            self.len += 1;
        }
        if self.pos == 0 {
            self.pos = self.capacity;
        }
        self.pos -= 1;

        self.update(&value)?;

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_timeseries() -> Result<()> {

        let mut ts = TimeseriesImpl::new_with_capacity(4);

        assert_eq!(ts.as_vec(), vec![]);
        assert_eq!(ts.len(), 0);

        ts.push(1)?;
        ts.push(2)?;
        assert_eq!(ts.as_vec(), vec![2, 1]);
        assert_eq!(ts.len(), 2);

        ts.push(3)?;
        ts.push(4)?;

        assert_eq!(ts.as_vec(), vec![4, 3, 2, 1]);
        assert_eq!(ts.len(), 4);

        
        ts.push(5)?;
        ts.push(6)?;
        ts.push(7)?;

        assert_eq!(ts.as_vec(), vec![7, 6, 5, 4]);
        assert_eq!(ts.len(), 4);

        ts.update(&8)?;
        assert_eq!(ts.as_vec(), vec![8, 6, 5, 4]);

        ts.update_with(&|val| { *val = 9; })?;
        assert_eq!(ts.as_vec(), vec![9, 6, 5, 4]);

        let x = 10;
        ts.update_with(&move |val| { *val = x; })?;
        assert_eq!(ts.as_vec(), vec![10, 6, 5, 4]);

        ts.push(11)?;
        ts.push(12)?;
        assert_eq!(ts.first(), Some(&12));

        ts.clear();
        assert_eq!(ts.as_vec(), vec![]);
        assert_eq!(ts.len(), 0);

        ts.push(1)?;
        ts.push(2)?;
        assert_eq!(ts.as_vec(), vec![2, 1]);
        assert_eq!(ts.len(), 2);

        assert_eq!(ts.as_ref_vec(), vec![&2, &1]);

        Ok(())
    }
}
