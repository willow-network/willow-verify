









use crate::Error;


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SorobanSuccess {

    pub tx: u32,

    pub tx_hash: [u8; 32],

    pub hash: [u8; 32],
}

const E: Error = Error::Xdr("TransactionResultSet");

struct R<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> R<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.p.checked_add(n).ok_or(E)?;
        let s = self.b.get(self.p..end).ok_or(E)?;
        self.p = end;
        Ok(s)
    }
    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn h32(&mut self) -> Result<[u8; 32], Error> {
        Ok(self.take(32)?.try_into().unwrap())
    }
    fn skip(&mut self, n: usize) -> Result<(), Error> {
        self.take(n).map(|_| ())
    }

    fn len(&mut self) -> Result<usize, Error> {
        let n = self.u32()? as usize;
        if n > (self.b.len() - self.p) / 4 {
            return Err(E);
        }
        Ok(n)
    }

    fn account(&mut self) -> Result<(), Error> {
        if self.i32()? != 0 {
            return Err(E);
        }
        self.skip(32)
    }

    fn asset(&mut self) -> Result<(), Error> {
        match self.i32()? {
            0 => Ok(()),
            1 => {
                self.skip(4)?;
                self.account()
            }
            2 => {
                self.skip(12)?;
                self.account()
            }
            _ => Err(E),
        }
    }

    fn claim_atom(&mut self) -> Result<(), Error> {
        match self.i32()? {
            0 => self.skip(32 + 8)?,
            1 => {
                self.account()?;
                self.skip(8)?;
            }
            2 => self.skip(32)?,
            _ => return Err(E),
        }
        self.asset()?;
        self.skip(8)?;
        self.asset()?;
        self.skip(8)
    }
    fn claim_atoms(&mut self) -> Result<(), Error> {
        for _ in 0..self.len()? {
            self.claim_atom()?;
        }
        Ok(())
    }

    fn manage_offer_success(&mut self) -> Result<(), Error> {
        self.claim_atoms()?;
        match self.i32()? {
            0 | 1 => {

                self.account()?;
                self.skip(8)?;
                self.asset()?;
                self.asset()?;
                self.skip(8 + 8 + 4)?;
                if self.i32()? != 0 {
                    return Err(E);
                }
                Ok(())
            }
            2 => Ok(()),
            _ => Err(E),
        }
    }


    fn op_result(&mut self) -> Result<Option<[u8; 32]>, Error> {
        match self.i32()? {
            0 => {}
            -6..=-1 => return Ok(None),
            _ => return Err(E),
        }
        let ty = self.i32()?;
        let code = self.i32()?;

        const MIN: [i32; 27] = [
            -4, -9, -12, -12, -12, -10, -8, -6, -7, -1, -4, -1, -12, -12, -5, -6, -3, -1, -5, -4,
            -3, -5, -8, -6, -5, -3, -3,
        ];
        let min = *MIN.get(usize::try_from(ty).map_err(|_| E)?).ok_or(E)?;
        if !(min..=0).contains(&code) {
            return Err(E);
        }
        match (ty, code) {

            (2 | 13, 0) => {
                self.claim_atoms()?;
                self.account()?;
                self.asset()?;
                self.skip(8)?;
            }
            (2 | 13, -9) => self.asset()?,

            (3 | 4 | 12, 0) => self.manage_offer_success()?,

            (8, 0) => self.skip(8)?,

            (9, 0) => {
                for _ in 0..self.len()? {
                    self.account()?;
                    self.skip(8)?;
                }
            }

            (14, 0) => {
                if self.i32()? != 0 {
                    return Err(E);
                }
                self.skip(32)?;
            }
            (24, 0) => return Ok(Some(self.h32()?)),
            _ => {}
        }
        Ok(None)
    }


    fn op_results(&mut self, keep: bool, out: &mut Vec<[u8; 32]>) -> Result<(), Error> {
        for _ in 0..self.len()? {
            if let Some(h) = self.op_result()? {
                if keep {
                    out.push(h);
                }
            }
        }
        Ok(())
    }
}


pub fn soroban_successes(bytes: &[u8]) -> Result<Vec<SorobanSuccess>, Error> {
    let mut r = R { b: bytes, p: 0 };
    let mut out = Vec::new();
    let mut hashes = Vec::new();
    for tx in 0..r.len()? as u32 {
        let tx_hash = r.h32()?;
        r.skip(8)?; // feeCharged
        hashes.clear();
        match r.i32()? {

            c @ (1 | -13) => {
                r.skip(32 + 8)?;
                match r.i32()? {
                    ic @ (0 | -1) => r.op_results(c == 1 && ic == 0, &mut hashes)?,
                    -12..=-2 | -18..=-14 => {}
                    _ => return Err(E),
                }
                if r.i32()? != 0 {
                    return Err(E);
                }
            }
            c @ (0 | -1) => r.op_results(c == 0, &mut hashes)?,
            -12..=-2 | -18..=-14 => {}
            _ => return Err(E),
        }
        if r.i32()? != 0 {
            return Err(E); // TransactionResult.ext
        }
        for h in hashes.drain(..) {
            out.push(SorobanSuccess {
                tx,
                tx_hash,
                hash: h,
            });
        }
    }
    if r.p != bytes.len() {
        return Err(E);
    }
    Ok(out)
}
