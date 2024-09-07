use twoway::find_bytes;


/// for finding specific pattern inside bytes and determine the start and the end of this pattern at parsed bytes
pub fn fast_finding_pattern_in_bytes(bytes:&[u8],pattern:&[u8])->Option<(usize,usize)>{
    let index = find_bytes(bytes,pattern);
    if let Some(index) = index {
        return  Some((index,index+pattern.len()-1));
    }
    None
}