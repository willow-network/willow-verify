




use alloy_rlp::{Decodable, Encodable};


pub(crate) struct ListWriter {
    payload: Vec<u8>,
}

impl ListWriter {
    pub(crate) fn new() -> Self {
        ListWriter {
            payload: Vec::with_capacity(640),
        }
    }

    pub(crate) fn item<T: Encodable + ?Sized>(&mut self, value: &T) -> &mut Self {
        value.encode(&mut self.payload);
        self
    }

    pub(crate) fn finish(&self) -> Vec<u8> {
        let head = alloy_rlp::Header {
            list: true,
            payload_length: self.payload.len(),
        };
        let mut out = Vec::with_capacity(self.payload.len() + 9);
        head.encode(&mut out);
        out.extend_from_slice(&self.payload);
        out
    }
}



pub(crate) fn next<T: Decodable>(payload: &mut &[u8]) -> Result<T, String> {
    T::decode(payload).map_err(|e| e.to_string())
}



pub(crate) fn open_list<'a>(buf: &mut &'a [u8]) -> Result<Option<&'a [u8]>, String> {
    let mut rest = *buf;
    let head = alloy_rlp::Header::decode(&mut rest).map_err(|e| e.to_string())?;
    if !head.list {
        return Ok(None);
    }
    if rest.len() < head.payload_length {
        return Err(alloy_rlp::Error::InputTooShort.to_string());
    }
    let (payload, after) = rest.split_at(head.payload_length);
    *buf = after;
    Ok(Some(payload))
}
