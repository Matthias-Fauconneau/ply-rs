use std::io::{ Write, Result, Error, ErrorKind };
use std::string::ToString;

use byteorder::{ BigEndian, LittleEndian, WriteBytesExt, ByteOrder };

use ply::*;

pub enum NewLine {
    N,
    R,
    RN
}

pub trait ToElement<P> {
    fn to_element(&self, element_def: &ElementDef) -> Result<DefaultElement>;
}

impl ToElement<DefaultElement> for DefaultElement {
    // simple identity
    fn to_element(&self, _props_def: &ElementDef) -> Result<DefaultElement> {
        Ok(self.clone())
    }
}


use std::marker::PhantomData;
pub struct Writer<P: ToElement<P>> {
    /// Should be fairly efficient, se `as_bytes()` in https://doc.rust-lang.org/src/collections/string.rs.html#1001
    new_line: String,
    phantom: PhantomData<P>,
}

impl<P: ToElement<P>> Writer<P> {
    pub fn new() -> Self {
        Writer {
            new_line: "\r\n".to_string(),
            phantom: PhantomData,
        }
    }
    pub fn set_newline(&mut self, new_line: NewLine) {
        self.new_line = match new_line {
            NewLine::R => "\r".to_string(),
            NewLine::N => "\n".to_string(),
            NewLine::RN => "\r\n".to_string(),
        };
    }
    // TODO: think about masking and valid/invalid symbols
    // TODO: make consistency check
    pub fn write_ply<T: Write>(&mut self, out: &mut T, ply: &Ply<P>) -> Result<usize> {
        let mut written = 0;
        written += try!(self.write_header(out, &ply.header));
        written += try!(self.write_payload(out, &ply.payload, &ply.header));
        out.flush().unwrap();
        Ok(written)
    }
    pub fn write_line_magic_number<T: Write>(&self, out: &mut T) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("ply".as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_format<T: Write>(&self, out: &mut T, encoding: &Encoding, version: &Version) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("format ".as_bytes()));
        written += try!(self.write_encoding(out, encoding));
        written += try!(out.write(format!(" {}.{}", version.major, version.minor).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_comment<T: Write>(&self, out: &mut T, comment: &Comment) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write(format!("comment {}", comment).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_obj_info<T: Write>(&self, out: &mut T, obj_info: &ObjInfo) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write(format!("obj_info {}", obj_info).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_element_definition<T: Write>(&self, out: &mut T, element: &ElementDef) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write(format!("element {} {}", element.name, element.count).as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_line_property_definition<T: Write>(&self, out: &mut T, property: &PropertyDef) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("property ".as_bytes()));
        written += try!(self.write_property_type(out, &property.data_type));
        written += try!(out.write(" ".as_bytes()));
        written += try!(out.write(property.name.as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    /// Writes the element line and all the property definitions
    pub fn write_element_definition<T: Write>(&self, out: &mut T, element: &ElementDef) -> Result<usize> {
        let mut written = 0;
        written += try!(self.write_line_element_definition(out, &element));
        for (_, p) in &element.properties {
            written += try!(self.write_line_property_definition(out, &p));
        }
        Ok(written)
    }
    pub fn write_line_end_header<T: Write>(&mut self, out: &mut T) -> Result<usize> {
        let mut written = 0;
        written += try!(out.write("end_header".as_bytes()));
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    pub fn write_header<T: Write>(&mut self, out: &mut T, header: &Header) -> Result<usize> {
        let mut written = 0;
        written += try!(self.write_line_magic_number(out));
        written += try!(self.write_line_format(out, &header.encoding, &header.version));
        for c in &header.comments {
            written += try!(self.write_line_comment(out, c));
        }
        for oi in &header.obj_infos {
            written += try!(self.write_line_obj_info(out, oi));
        }
        for (_, e) in &header.elements {
            written += try!(self.write_element_definition(out, &e));
        }
        written += try!(self.write_line_end_header(out));
        Ok(written)
    }

    fn write_encoding<T: Write>(&self, out: &mut T, encoding: &Encoding) -> Result<usize> {
        let s = match *encoding {
            Encoding::Ascii => "ascii",
            Encoding::BinaryBigEndian => "binary_big_endian",
            Encoding::BinaryLittleEndian => "binary_little_endian",
        };
        out.write(s.as_bytes())
    }
    fn write_property_type<T: Write>(&self, out: &mut T, data_type: &PropertyType) -> Result<usize> {
        match *data_type {
            PropertyType::Char => out.write("char".as_bytes()),
            PropertyType::UChar => out.write("uchar".as_bytes()),
            PropertyType::Short => out.write("short".as_bytes()),
            PropertyType::UShort => out.write("ushort".as_bytes()),
            PropertyType::Int => out.write("int".as_bytes()),
            PropertyType::UInt => out.write("uint".as_bytes()),
            PropertyType::Float => out.write("float".as_bytes()),
            PropertyType::Double => out.write("double".as_bytes()),
            PropertyType::List(ref index_type, ref t) => {
                let mut written = try!(out.write("list ".as_bytes()));
                match **index_type {
                    PropertyType::Float => return Err(Error::new(ErrorKind::InvalidInput, "List index can not be of type float.")),
                    PropertyType::Double => return Err(Error::new(ErrorKind::InvalidInput, "List index can not be of type double.")),
                    PropertyType::List(_, _) => return Err(Error::new(ErrorKind::InvalidInput, "List index can not be of type list.")),
                    _ => (),
                };
                written += try!(self.write_property_type(out, index_type));
                written += try!(out.write(" ".as_bytes()));
                written += try!(self.write_property_type(out, t));
                Ok(written)
            }
        }
    }
    ///// Payload
    pub fn write_payload<T: Write>(&mut self, out: &mut T, payload: &Payload<P>, header: &Header) -> Result<usize> {
        let mut written = 0;
        let element_defs = &header.elements;
        for (k, element_list) in payload {
            let element_def = &element_defs[k];
            written += try!(self.write_payload_of_element(out, element_list, element_def, header));
        }
        Ok(written)
    }
    pub fn write_payload_of_element<T: Write>(&mut self, out: &mut T, element_list: &Vec<P>, element_def: &ElementDef, header: &Header) -> Result<usize> {
        let mut written = 0;
        match header.encoding {
            Encoding::Ascii => for e in element_list {
                let raw_element = try!(e.to_element(element_def));
                written += try!(self.__write_ascii_element(out, &raw_element));
            },
            Encoding::BinaryBigEndian => for e in element_list {
                let raw_element = try!(e.to_element(element_def));
                written += try!(self.__write_binary_element::<T, BigEndian>(out, &raw_element, &element_def));
            },
            Encoding::BinaryLittleEndian => for e in element_list {
                let raw_element = try!(e.to_element(element_def));
                written += try!(self.__write_binary_element::<T, LittleEndian>(out, &raw_element, &element_def));
            }
        }
        Ok(written)
    }
    pub fn write_ascii_element<T: Write>(&self, out: &mut T, element: &P, element_def: &ElementDef) -> Result<usize> {
        let raw_element = try!(element.to_element(element_def));
        self.__write_ascii_element(out, &raw_element)
    }
    pub fn write_big_endian_element<T: Write> (&self, out: &mut T, element: &P, element_def: &ElementDef) -> Result<usize> {
        let raw_element = try!(element.to_element(element_def));
        self.__write_binary_element::<T, BigEndian>(out, &raw_element, element_def)
    }
    pub fn write_little_endian_element<T: Write> (&self, out: &mut T, element: &P, element_def: &ElementDef) -> Result<usize> {
        let raw_element = try!(element.to_element(element_def));
        self.__write_binary_element::<T, BigEndian>(out, &raw_element, element_def)
    }

    // private payload
    fn __write_binary_element<T: Write, B: ByteOrder>(&self, out: &mut T, element: &DefaultElement, element_def: &ElementDef) -> Result<usize> {
        let mut written = 0;
        for (k, property) in element {
            written += try!(self.__write_binary_property::<T, B>(out, property, &element_def.properties[k].data_type));
        }
        Ok(written)
    }
    fn __write_binary_property<T: Write, B: ByteOrder>(&self, out: &mut T, property: &Property, property_type: &PropertyType) -> Result<usize> {
         let result: usize = match *property {
            Property::Char(ref v) => {try!(out.write_i8(*v)); 1},
            Property::UChar(ref v) => {try!(out.write_u8(*v)); 1},
            Property::Short(ref v) => {try!(out.write_i16::<B>(*v)); 2},
            Property::UShort(ref v) => {try!(out.write_u16::<B>(*v)); 2},
            Property::Int(ref v) => {try!(out.write_i32::<B>(*v)); 4},
            Property::UInt(ref v) => {try!(out.write_u32::<B>(*v)); 4},
            Property::Float(ref v) => {try!(out.write_f32::<B>(*v)); 4},
            Property::Double(ref v) => {try!(out.write_f64::<B>(*v)); 8},
            Property::List(ref v) => {
                let mut written = 0;
                let index_type = match *property_type {
                    PropertyType::List(ref i, _) => i,
                    _ => return Err(Error::new(ErrorKind::InvalidInput, "Property definition must be of type List.")),
                };
                let vl = v.len();
                written += match **index_type {
                    PropertyType::Char => {try!(out.write_i8(vl as i8)); 1},
                    PropertyType::UChar => {try!(out.write_u8(vl as u8)); 1}
                    PropertyType::Short => {try!(out.write_i16::<B>(vl as i16)); 2},
                    PropertyType::UShort => {try!(out.write_u16::<B>(vl as u16)); 2},
                    PropertyType::Int => {try!(out.write_i32::<B>(vl as i32)); 4}
                    PropertyType::UInt => {try!(out.write_u32::<B>(vl as u32)); 4},
                    PropertyType::Float => return Err(Error::new(ErrorKind::InvalidInput, "List index must have integer type, Float found.")),
                    PropertyType::Double => return Err(Error::new(ErrorKind::InvalidInput, "List index must have integer type, Double found.")),
                    PropertyType::List(_,_) => return Err(Error::new(ErrorKind::InvalidInput, "List index must have integer type, List found.")),
                };
                for e in v {
                    written += try!(self.__write_binary_property::<T, B>(out, &e, &*index_type));
                }
                written as usize
            },
        };
        Ok(result)
    }
    fn __write_ascii_element<T: Write>(&self, out: &mut T, element: &DefaultElement) -> Result<usize> {
        let mut written = 0;
        let mut p_iter = element.iter();
        let (_name, prop_val) = p_iter.next().unwrap();
        written += try!(self.write_ascii_property(out, prop_val));
        loop {
            written += try!(out.write(" ".as_bytes()));
            let n = p_iter.next();
            if n == None {
                break;
            }
            let (_name, prop_val) = n.unwrap();
            written += try!(self.write_ascii_property(out, prop_val));
        }
        written += try!(self.write_new_line(out));
        Ok(written)
    }
    fn write_ascii_property<T: Write>(&self, out: &mut T, data_element: &Property) -> Result<usize> {
         let result = match *data_element {
            Property::Char(ref v) => self.write_simple_value(v, out),
            Property::UChar(ref v) => self.write_simple_value(v, out),
            Property::Short(ref v) => self.write_simple_value(v, out),
            Property::UShort(ref v) => self.write_simple_value(v, out),
            Property::Int(ref v) => self.write_simple_value(v, out),
            Property::UInt(ref v) => self.write_simple_value(v, out),
            Property::Float(ref v) => self.write_simple_value(v, out),
            Property::Double(ref v) => self.write_simple_value(v, out),
            Property::List(ref v) => {
                let mut written = 0;
                written += try!(out.write(&v.len().to_string().as_bytes()));
                for e in v {
                    written += try!(out.write(" ".as_bytes()));
                    written += try!(self.write_ascii_property(out, &e));
                }
                Ok(written)
            },
        };
        result
    }

    fn write_new_line<T: Write>(&self, out: &mut T) -> Result<usize> {
        out.write(self.new_line.as_bytes())
    }
    fn write_simple_value<T: Write, V: ToString>(&self, value: &V, out: &mut T) -> Result<usize> {
        out.write(value.to_string().as_bytes())
    }
}
