use crate::resp::value::DataObjects;

pub fn resp_decode_value(byte_array: &[u8], iterator_var: &mut usize) -> Option<DataObjects> {
    // let mut data_array: Vec<DataObjects> = Vec::new();
    // let mut iterator_var = 0;
    let stt = str::from_utf8(byte_array);
    let n = byte_array.len();
    while *iterator_var < n {
        match byte_array[*iterator_var] as char {
            '+'  => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let new_val = DataObjects::BasicString(str::from_utf8(&byte_array[*iterator_var+1..iterator_end]).unwrap().to_string());
                *iterator_var=iterator_end+2;
                return Some(new_val);
            },
            '-' => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let new_val = DataObjects::Error(str::from_utf8(&byte_array[*iterator_var+1..iterator_end]).unwrap().to_string());
                *iterator_var=iterator_end+2;
                return Some(new_val);
            },
            ':' => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let subslice = &byte_array[*iterator_var+1..iterator_end];
                let new_val = DataObjects::Integer(str::from_utf8(subslice).unwrap().parse().unwrap());
                *iterator_var=iterator_end+2;
                return Some(new_val);
            },
            '$' => {
                let mut iterator_end = *iterator_var;
                while (iterator_end < n-1) {
                    if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
                    else {iterator_end+=1;}
                }
                let subslice = &byte_array[*iterator_var+1..iterator_end];
                let length_buffer: i32 = str::from_utf8(subslice).unwrap().parse().unwrap();
                let new_val = match length_buffer{
                    -1 => DataObjects::BulkString(None),
                    _ => DataObjects::BulkString(Some(byte_array[iterator_end+2..iterator_end+2+length_buffer as usize].to_vec()))
                };
                *iterator_var=iterator_end + 4 + length_buffer as usize;
                return Some(new_val);
            },
            '*' => {

                return resp_decode_array(byte_array, iterator_var);
            },
            _ => {
                //println!("TF?????, {}", iterator_var);
            }
        }
    }
    return None;
}

pub fn resp_decode_array(byte_array: &[u8], iterator_var: &mut usize) -> Option<DataObjects> {
    let mut vc: Vec<DataObjects> = Vec::new();
    let mut iterator_end = *iterator_var;
    let n = byte_array.len();
    while (iterator_end < n-1) {
        if byte_array[iterator_end] as char == '\r' && byte_array[iterator_end+1] as char == '\n' {break;}
        else {iterator_end+=1;}
    }
    let subslice = &byte_array[*iterator_var+1..iterator_end];
    let length_buffer: i32 = str::from_utf8(subslice).unwrap().parse().unwrap();
    *iterator_var = iterator_end+2;
    if length_buffer == 0 {
        return Some(DataObjects::Array(Some(vc)));
    } else if length_buffer < 0 {
        return Some(DataObjects::Array(None));
    }
    for i in 0..length_buffer {
        let val = resp_decode_value(byte_array, iterator_var);
        if let Some(x) = val {
            vc.push(x);
        }
    }
    return Some(DataObjects::Array(Some(vc)));
}
