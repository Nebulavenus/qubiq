use std::io::{Read, Write};

pub fn read_byte<R: Read>(reader: &mut R) -> anyhow::Result<u8> {
    let mut buf = [0u8];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub fn read_sbyte<R: Read>(reader: &mut R) -> anyhow::Result<i8> {
    let mut buf = [0u8];
    reader.read_exact(&mut buf)?;
    Ok(buf[0] as i8)
}

pub fn read_short<R: Read>(reader: &mut R) -> anyhow::Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

pub fn read_int<R: Read>(reader: &mut R) -> anyhow::Result<i32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

pub fn read_long<R: Read>(reader: &mut R) -> anyhow::Result<i64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(i64::from_be_bytes(buf))
}

pub fn read_float<R: Read>(reader: &mut R) -> anyhow::Result<f32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(f32::from_be_bytes(buf))
}

pub fn read_double<R: Read>(reader: &mut R) -> anyhow::Result<f64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

pub fn read_mcstring<R: Read>(reader: &mut R) -> anyhow::Result<String> {
    let mut buf = [0u8; 64];
    reader.read_exact(&mut buf)?;
    let res = String::from_utf8_lossy(&buf);
    Ok(res.into())
}

pub fn read_utfstring<R: Read>(reader: &mut R) -> anyhow::Result<String> {
    let size = read_short(reader)?;
    if size > 0 {
        let mut buf = vec![0u8; size as usize];
        reader.read_exact(&mut buf)?;
        let res = String::from_utf8(buf)?;
        Ok(res)
    } else {
        Ok(String::new())
    }
}

pub fn read_bytearray<R: Read>(reader: &mut R, count: usize) -> anyhow::Result<Vec<i8>> {
    let mut buf = vec![0u8; count];
    reader.read_exact(&mut buf)?;
    Ok(buf.iter().map(|x| *x as i8).collect())
}

pub fn write_byte<W: Write>(writer: &mut W, val: u8) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

pub fn write_sbyte<W: Write>(writer: &mut W, val: i8) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

pub fn write_short<W: Write>(writer: &mut W, val: i16) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

pub fn write_int<W: Write>(writer: &mut W, val: i32) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

pub fn write_long<W: Write>(writer: &mut W, val: i64) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

pub fn write_float<W: Write>(writer: &mut W, val: f32) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

pub fn write_double<W: Write>(writer: &mut W, val: f64) -> anyhow::Result<()> {
    writer.write(&val.to_be_bytes())?;
    Ok(())
}

pub fn write_mcstring<W: Write>(writer: &mut W, val: String) -> anyhow::Result<()> {
    let mut buf = [0u8; 64];
    let val_bytes = val.as_bytes();
    let vb_len = val_bytes.len();
    if vb_len > buf.len() {
        buf.clone_from_slice(&val_bytes[..64]);
    } else {
        buf[..vb_len].clone_from_slice(&val_bytes);
    }
    writer.write(&buf)?;
    Ok(())
}

pub fn write_utfstring<W: Write>(writer: &mut W, val: String) -> anyhow::Result<()> {
    let size = val.len() as i16;
    write_short(writer, size)?;

    if size != 0 {
        writer.write_all(val.as_bytes())?;
    }
    Ok(())
}

pub fn write_bytearray<W: Write>(writer: &mut W, val: Vec<i8>) -> anyhow::Result<()> {
    let res = val.iter().map(|v| *v as u8).collect::<Vec<_>>();
    writer.write_all(&res)?;
    Ok(())
}
