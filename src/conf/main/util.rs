use bytes::Bytes;
use failure::Error;
use std::mem;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub enum RefVal<T>{
    Ref(Bytes),
    Val(T),
}

impl<T: Clone> RefVal<T> {
    pub fn get_ref(&self)-> Option<Bytes> {
        match self {
            RefVal::Ref(x) => Some(x.clone()),
            _ => None,
        }
    }
    pub fn insert_value(&mut self, valmap: &BTreeMap<Bytes, T>)->Result<(), Error> {
        if let Some(n) = self.get_ref() {
            let g = valmap.get(&n)
                .ok_or_else(|| format_err!("Variable {:?} is not defined", n))?;
            mem::replace(self, RefVal::Val(g.clone()));
        }
        Ok(())
    }
}
impl<T> RefVal<T> {
    pub fn val(&self)->&T {
        match self {
            RefVal::Val(v) => &v,
            RefVal::Ref(n) => panic!("{:?} is not value", n),
        }
    }
}
