#![allow(bad_style, dead_code)]

use std::error::Error;
use std::fmt;

pub const MAX_COMPRESSION_INPUT_SIZE: u32 = 0x7E00_0000;

/// Decode a block. Assumptions: input contains all sequences of a
/// chunk. If the returned value is an error then an error occurred
/// at the returned offset. If the return value is `Ok` then it is
/// the number of decoded bytes.
///
/// This method's code was taken from node-lz4 by Pierre Curto. MIT license.
pub fn decompress(input: &[u8], output: &mut Vec<u8>) -> Result<usize, usize> {
    struct Output<'a> {
        data: &'a mut Vec<u8>,
        start_index: usize,
    }
    impl<'a> Output<'a> {
        pub fn new(data: &'a mut Vec<u8>) -> Self {
            let start_index = data.len();
            Self { data, start_index }
        }
        pub fn push(&mut self, value: u8) {
            self.data.push(value)
        }
        pub fn decoded_count(&self) -> usize {
            self.data.len() - self.start_index
        }
        pub fn get(&self, index: usize) -> Option<&u8> {
            self.data.get(index)
        }
    }
    let mut output = Output::new(output);

    // Process each sequence in the incoming data
    let mut i = 0;
    while i < input.len() {
        let token = input[i];
        i += 1;

        // Literals
        let mut literals_length: usize = (token as usize) >> 4;
        if literals_length > 0 {
            // length of literals
            let mut l = literals_length + 240;
            while l == 255 {
                l = input[i] as usize;
                i += 1;
                literals_length += l;
            }

            // Copy the literals
            let end = i + literals_length;
            while i < end {
                output.push(input[i]);
                i += 1;
            }

            // End of buffer?
            if i == input.len() {
                break;
            }
        }

        // Match copy
        // 2 bytes offset (little endian)
        let mut offset = input[i] as usize;
        i += 1;
        offset |= (input[i] as usize) << 8;
        i += 1;

        // 0 is an invalid offset value
        if offset == 0 || offset > output.decoded_count() {
            return Err((i as usize) - 2);
        }

        // length of match copy
        let mut match_length = (token & 0xf) as usize;
        let mut l = match_length + 240;
        while l == 255 {
            l = input[i] as usize;
            i += 1;
            match_length += l;
        }

        // Copy the match
        let mut pos = output.decoded_count() - offset; // position of the match copy in the current output
        let end = output.decoded_count() + match_length + 4; // minmatch = 4
        while output.decoded_count() < end {
            if let Some(value) = output.get(pos) {
                let value = *value;
                output.push(value);
            } else {
                return Err(i);
            }
            pos += 1;
        }
    }

    Ok(output.decoded_count())
}

/// Returns the maximum length of a lz4 block, given it's uncompressed length.
pub fn compress_bound(uncompressed_size: u32) -> Option<u32> {
    if uncompressed_size > MAX_COMPRESSION_INPUT_SIZE {
        None
    } else {
        Some(uncompressed_size + (uncompressed_size / 255) + 16)
    }
}

const MIN_MATCH: usize = 4;
const COPY_LENGTH: usize = 8;
const MF_LIMIT: usize = COPY_LENGTH + MIN_MATCH;

pub trait CompressInput {
    fn expected_len(&self) -> Option<usize>;
}
pub trait CompressOutput {
    fn write(data: u8) -> Result<(), Box<dyn Error + 'static>>;
    fn expect_len(len: usize) -> Result<(), Box<dyn Error + 'static>>;
}
pub trait CompressHashTable {}

#[derive(Debug)]
pub enum CompressError {
    OutputTooSmall {
        grow_error: Box<dyn Error + 'static>,
        wanted_len: usize,
    },
    InputTooLarge(usize),
}
impl fmt::Display for CompressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompressError::InputTooLarge(expected_len) => write!(
                f,
                "Expected input of {} bytes is too large. Max size is {} bytes.",
                expected_len, MAX_COMPRESSION_INPUT_SIZE
            ),
            CompressError::OutputTooSmall { wanted_len, .. } => {
                write!(f, "Output couldn't grow to fit: {} bytes.", wanted_len)
            }
        }
    }
}
impl Error for CompressError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CompressError::InputTooLarge(_) => None,
            CompressError::OutputTooSmall { grow_error, .. } => Some(&**grow_error),
        }
    }
}
/*
pub fn compress<I, O, H>(src: I, dst: O, hash_table: H) -> Result<(), CompressError>
    where
        I: CompressInput,
        O: CompressOutput,
        H: CompressHashTable,
{
    let src_length = src.expected_len();
    var dpos = sIdx
    var dlen = eIdx - sIdx
    var anchor = 0

    if src_length >= MAX_COMPRESSION_INPUT_SIZE {
        return Err(CompressError::InputTooLarge(src_length));
    }

    // Minimum of input bytes for compression (LZ4 specs)
    if (src_length > mfLimit) {
        var n = exports.compressBound(src_length)
        if ( dlen < n ) throw Error("output too small: " + dlen + " < " + n)

        var
            step  = 1
        ,	findMatchAttempts = (1 << skipStrength) + 3
        // Keep last few bytes incompressible (LZ4 specs):
        // last 5 bytes must be literals
        ,	srcLength = src.length - mfLimit

        while (pos + minMatch < srcLength) {
            // Find a match
            // min match of 4 bytes aka sequence
            var sequenceLowBits = src[pos+1]<<8 | src[pos]
            var sequenceHighBits = src[pos+3]<<8 | src[pos+2]
            // compute hash for the current sequence
            var hash = Math.imul(sequenceLowBits | (sequenceHighBits << 16), hasher) >>> hashShift
            // get the position of the sequence matching the hash
            // NB. since 2 different sequences may have the same hash
            // it is double-checked below
            // do -1 to distinguish between initialized and uninitialized values
            var ref = hashTable[hash] - 1
            // save position of current sequence in hash table
            hashTable[hash] = pos + 1

            // first reference or within 64k limit or current sequence !== hashed one: no match
            if ( ref < 0 ||
                ((pos - ref) >>> 16) > 0 ||
                (
                    ((src[ref+3]<<8 | src[ref+2]) != sequenceHighBits) ||
                    ((src[ref+1]<<8 | src[ref]) != sequenceLowBits )
                )
            ) {
                // increase step if nothing found within limit
                step = findMatchAttempts++ >> skipStrength
                pos += step
                continue
            }

            findMatchAttempts = (1 << skipStrength) + 3

            // got a match
            var literals_length = pos - anchor
            var offset = pos - ref

            // minMatch already verified
            pos += minMatch
            ref += minMatch

            // move to the end of the match (>=minMatch)
            var match_length = pos
            while (pos < srcLength && src[pos] == src[ref]) {
                pos++
                ref++
            }

            // match length
            match_length = pos - match_length

            // token
            var token = match_length < mlMask ? match_length : mlMask

            // encode literals length
            if (literals_length >= runMask) {
                // add match length to the token
                dst[dpos++] = (runMask << mlBits) + token
                for (var len = literals_length - runMask; len > 254; len -= 255) {
                    dst[dpos++] = 255
                }
                dst[dpos++] = len
            } else {
                // add match length to the token
                dst[dpos++] = (literals_length << mlBits) + token
            }

            // write literals
            for (var i = 0; i < literals_length; i++) {
                dst[dpos++] = src[anchor+i]
            }

            // encode offset
            dst[dpos++] = offset
            dst[dpos++] = (offset >> 8)

            // encode match length
            if (match_length >= mlMask) {
                match_length -= mlMask
                while (match_length >= 255) {
                    match_length -= 255
                    dst[dpos++] = 255
                }

                dst[dpos++] = match_length
            }

            anchor = pos
        }
    }

    // cannot compress input
    if (anchor == 0) return 0

    // Write last literals
    // encode literals length
    literals_length = src.length - anchor
    if (literals_length >= runMask) {
        // add match length to the token
        dst[dpos++] = (runMask << mlBits)
        for (var ln = literals_length - runMask; ln > 254; ln -= 255) {
            dst[dpos++] = 255
        }
        dst[dpos++] = ln
    } else {
        // add match length to the token
        dst[dpos++] = (literals_length << mlBits)
    }

    // write literals
    pos = anchor
    while (pos < src.length) {
        dst[dpos++] = src[pos++]
    }

return dpos
}*/
