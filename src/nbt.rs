use crate::util::*;
use std::collections::HashMap;
use std::io::{Read, Write};

#[derive(Clone, Debug)]
pub enum Tag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<Tag>),
    Compound(HashMap<String, Tag>),
}

impl Tag {
    pub fn kind(&self) -> i8 {
        match self {
            Tag::End => 0x00,
            Tag::Byte(_) => 0x01,
            Tag::Short(_) => 0x02,
            Tag::Int(_) => 0x03,
            Tag::Long(_) => 0x04,
            Tag::Float(_) => 0x05,
            Tag::Double(_) => 0x06,
            Tag::ByteArray(_) => 0x07,
            Tag::String(_) => 0x08,
            Tag::List(_) => 0x09,
            Tag::Compound(_) => 0x0a,
        }
    }

    fn read<R: Read>(reader: &mut R, tag_kind: i8) -> anyhow::Result<Tag> {
        match tag_kind {
            0x00 => Ok(Tag::End),
            0x01 => Ok(Tag::Byte(read_sbyte(reader)?)),
            0x02 => Ok(Tag::Short(read_short(reader)?)),
            0x03 => Ok(Tag::Int(read_int(reader)?)),
            0x04 => Ok(Tag::Long(read_long(reader)?)),
            0x05 => Ok(Tag::Float(read_float(reader)?)),
            0x06 => Ok(Tag::Double(read_double(reader)?)),
            0x07 => {
                let count = read_int(reader)? as usize;
                Ok(Tag::ByteArray(read_bytearray(reader, count)?))
            }
            0x08 => Ok(Tag::String(read_utfstring(reader)?)),
            0x09 => {
                let tag_kind = read_sbyte(reader)?;

                let count = read_int(reader)? as usize;
                let mut list = Vec::with_capacity(count);

                for _ in 0..count {
                    list.push(Tag::read(reader, tag_kind)?);
                }

                Ok(Tag::List(list))
            }
            0x0a => {
                let mut m = HashMap::new();
                loop {
                    let tag_kind = read_sbyte(reader)?;

                    if tag_kind == 0x00 {
                        break;
                    }

                    let key = read_utfstring(reader)?;
                    let tag = Tag::read(reader, tag_kind)?;
                    m.insert(key, tag);
                }
                Ok(Tag::Compound(m))
            }
            _ => Err(anyhow::anyhow!("Unknown tag kind!")),
        }
    }

    fn write<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        match *self {
            Tag::End => Ok(()),
            Tag::Byte(v) => write_sbyte(writer, v),
            Tag::Short(v) => write_short(writer, v),
            Tag::Int(v) => write_int(writer, v),
            Tag::Long(v) => write_long(writer, v),
            Tag::Float(v) => write_float(writer, v),
            Tag::Double(v) => write_double(writer, v),
            Tag::ByteArray(ref v) => {
                write_int(writer, v.len() as i32)?;
                write_bytearray(writer, v.clone())?;
                Ok(())
            }
            Tag::String(ref s) => write_utfstring(writer, s.clone()),
            Tag::List(ref v) => {
                let tag_kind = v.get(0).map(Tag::kind).unwrap_or(1);

                write_sbyte(writer, tag_kind)?;
                write_int(writer, v.len() as i32)?;

                for tag in v.iter() {
                    tag.write(writer)?;
                }
                Ok(())
            }
            Tag::Compound(ref v) => {
                for (key, tag) in v.iter() {
                    let tag_kind = tag.kind();

                    write_sbyte(writer, tag_kind)?;
                    write_utfstring(writer, key.clone())?;
                    tag.write(writer)?;
                }
                write_sbyte(writer, 0x00)?;
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct NBT {
    key: String,
    tag: Tag,
}

impl From<Tag> for NBT {
    fn from(tag: Tag) -> Self {
        NBT {
            key: String::new(),
            tag,
        }
    }
}

impl NBT {
    pub fn new<T: AsRef<str>>(key: T, tag: Tag) -> Self {
        let key = key.as_ref().into();
        NBT { key, tag }
    }

    pub fn read<R: Read>(reader: &mut R) -> anyhow::Result<NBT> {
        let tag_kind = read_sbyte(reader)?;

        if tag_kind != 0x0a {
            return Err(anyhow::anyhow!("Invalid tag kind, expected compound"));
        }

        let key = read_utfstring(reader)?;
        let tag = Tag::read(reader, tag_kind)?;

        Ok(NBT { key, tag })
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        write_sbyte(writer, 0x0a)?;
        write_utfstring(writer, self.key.clone())?;
        self.tag.write(writer)?;
        write_sbyte(writer, 0x00)?;

        Ok(())
    }

    pub fn key(&self) -> &str {
        self.key.as_ref()
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }
}
