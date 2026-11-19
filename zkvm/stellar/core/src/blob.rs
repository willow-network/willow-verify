























use crate::Error;


pub const ANCHOR_MAGIC: u32 = 0x3141_5357; // "WSA1"































#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorWitness {
    pub network_id: [u8; 32],
    pub ledger: u32,
    pub qset: Vec<u8>,
    pub next_qset: Vec<u8>,
    pub header: Vec<u8>,
    pub tx_set_next: Vec<u8>,
    pub envelopes: Vec<Vec<u8>>,
}

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.p.checked_add(n).ok_or(Error::Frame("length"))?;
        if end > self.b.len() {
            return Err(Error::Frame("short read"));
        }
        let s = &self.b[self.p..end];
        self.p = end;
        Ok(s)
    }



    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn h32(&mut self) -> Result<[u8; 32], Error> {
        Ok(self.take(32)?.try_into().unwrap())
    }
    fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }















    fn end(&self) -> Result<(), Error> {
        if self.p != self.b.len() {
            return Err(Error::Frame("trailing bytes"));
        }
        Ok(())
    }
}

fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}



















































































impl AnchorWitness {
    pub fn decode(blob: &[u8]) -> Result<Self, Error> {
        let mut r = Rd { b: blob, p: 0 };
        if r.u32()? != ANCHOR_MAGIC {
            return Err(Error::Frame("magic"));
        }
        let network_id = r.h32()?;
        let ledger = r.u32()?;
        let qset = r.bytes()?;
        let next_qset = r.bytes()?;
        let header = r.bytes()?;
        let tx_set_next = r.bytes()?;
        let n = r.u32()?;
        let mut envelopes = Vec::new();
        for _ in 0..n {
            envelopes.push(r.bytes()?);
        }
        r.end()?;
        Ok(AnchorWitness {
            network_id,
            ledger,
            qset,
            next_qset,
            header,
            tx_set_next,
            envelopes,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&ANCHOR_MAGIC.to_le_bytes());
        out.extend_from_slice(&self.network_id);
        out.extend_from_slice(&self.ledger.to_le_bytes());
        put_bytes(&mut out, &self.qset);
        put_bytes(&mut out, &self.next_qset);
        put_bytes(&mut out, &self.header);
        put_bytes(&mut out, &self.tx_set_next);
        out.extend_from_slice(&(self.envelopes.len() as u32).to_le_bytes());
        for e in &self.envelopes {
            put_bytes(&mut out, e);
        }
        out
    }
}
