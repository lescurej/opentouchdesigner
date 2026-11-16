
struct HoistedArray {
    name: String,
    cols: usize,
    is_int: bool,
    base: usize,
}

pub(super) fn hoist_const_arrays(source: &str) -> (String, Vec<f32>) {
    let defines = int_defines(source);
    let mut stripped = String::new();
    let mut values = Vec::new();
    let mut arrays = Vec::new();
    let mut i = 0;
    let bytes = source.as_bytes();
    while i < bytes.len() {
        if bytes[i..].starts_with(b"const ") && !in_line_comment(source, i) {
            if let Some(parsed) = parse_const_array(&source[i..], &defines) {
                if parsed.values.len() == parsed.rows * parsed.cols.max(1) {
                    arrays.push(HoistedArray {
                        name: parsed.name,
                        cols: parsed.cols,
                        is_int: parsed.is_int,
                        base: values.len(),
                    });
                    values.extend(parsed.values);
                    i += parsed.consumed;
                    continue;
                }
            }
        }
        stripped.push(source[i..].chars().next().unwrap_or('\0'));
        i += source[i..].chars().next().unwrap_or('\0').len_utf8();
    }
    arrays.sort_by_key(|a| std::cmp::Reverse(a.name.len()));
    for array in arrays {
        stripped = replace_array_uses(&stripped, &array);
    }
    (stripped, values)
}

fn int_defines(source: &str) -> std::collections::HashMap<String, usize> {
    let mut out = std::collections::HashMap::new();
    for line in source.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("#define ") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(value) = parts.next() else { continue };
        if parts.next().is_some() {
            continue;
        }
        if let Ok(n) = value.parse::<usize>() {
            out.insert(name.to_string(), n);
        }
    }
    out
}

struct ParsedArray {
    consumed: usize,
    name: String,
    rows: usize,
    cols: usize,
    is_int: bool,
    values: Vec<f32>,
}

fn parse_const_array(
    source: &str,
    defines: &std::collections::HashMap<String, usize>,
) -> Option<ParsedArray> {
    let bytes = source.as_bytes();
    if !bytes.starts_with(b"const ") {
        return None;
    }
    let mut i = 6;
    i = skip_ws(source, i);
    let (ty, j) = read_ident(source, i)?;
    let is_int = match ty.as_str() {
        "float" => false,
        "int" => true,
        _ => return None,
    };
    i = skip_ws(source, j);
    let (name, j) = read_ident(source, i)?;
    i = j;
    if bytes.get(i) != Some(&b'[') {
        return None;
    }
    let (rows, j) = read_bracket_size(source, i, defines)?;
    i = skip_ws(source, j);
    let (cols, j) = if bytes.get(i) == Some(&b'[') {
        read_bracket_size(source, i, defines)?
    } else {
        (1, i)
    };
    i = skip_ws(source, j);
    if bytes.get(i) != Some(&b'=') {
        return None;
    }
    i += 1;
    let open = source[i..].find('(')? + i;
    let (values, after) = read_initializer_numbers(source, open)?;
    let mut end = skip_ws(source, after);
    if bytes.get(end) != Some(&b';') {
        return None;
    }
    end += 1;
    Some(ParsedArray {
        consumed: end,
        name,
        rows,
        cols,
        is_int,
        values,
    })
}

fn read_initializer_numbers(source: &str, open: usize) -> Option<(Vec<f32>, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return None;
    }
    let mut i = open;
    let mut depth = 0i32;
    let mut bracket = 0i32;
    let mut values = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'[' => {
                bracket += 1;
                i += 1;
            }
            b']' => {
                bracket -= 1;
                i += 1;
            }
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some((values, i));
                }
            }
            b'-' | b'0'..=b'9' | b'.' if bracket == 0 => {
                let (n, next) = read_number(source, i)?;
                values.push(n);
                i = next;
            }
            _ => i += 1,
        }
    }
    None
}

fn read_number(source: &str, i: usize) -> Option<(f32, usize)> {
    let bytes = source.as_bytes();
    let start = i;
    let mut j = i;
    if bytes.get(j) == Some(&b'-') {
        j += 1;
    }
    let digits = j;
    while matches!(bytes.get(j), Some(b'0'..=b'9')) {
        j += 1;
    }
    if bytes.get(j) == Some(&b'.') {
        j += 1;
        while matches!(bytes.get(j), Some(b'0'..=b'9')) {
            j += 1;
        }
    }
    if j == digits || (bytes.get(start) == Some(&b'-') && j == start + 1) {
        return None;
    }
    if matches!(bytes.get(j), Some(b'e' | b'E')) {
        let exp = j;
        j += 1;
        if matches!(bytes.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let exp_digits = j;
        while matches!(bytes.get(j), Some(b'0'..=b'9')) {
            j += 1;
        }
        if j == exp_digits {
            j = exp;
        }
    }
    let n = source[start..j].parse::<f32>().ok()?;
    Some((n, j))
}

fn read_bracket_size(
    source: &str,
    i: usize,
    defines: &std::collections::HashMap<String, usize>,
) -> Option<(usize, usize)> {
    if source.as_bytes().get(i) != Some(&b'[') {
        return None;
    }
    let end = i + 1 + source[i + 1..].find(']')?;
    let token = source[i + 1..end].trim();
    let n = if let Ok(n) = token.parse::<usize>() {
        n
    } else {
        *defines.get(token)?
    };
    Some((n, end + 1))
}

fn read_ident(source: &str, i: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let start = *bytes.get(i)?;
    if !(start == b'_' || start.is_ascii_alphabetic()) {
        return None;
    }
    let mut j = i + 1;
    while matches!(bytes.get(j), Some(c) if *c == b'_' || c.is_ascii_alphanumeric()) {
        j += 1;
    }
    Some((source[i..j].to_string(), j))
}

fn skip_ws(source: &str, mut i: usize) -> usize {
    let bytes = source.as_bytes();
    while matches!(bytes.get(i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        i += 1;
    }
    i
}

fn in_line_comment(source: &str, i: usize) -> bool {
    let line = source[..i].rfind('\n').map(|p| p + 1).unwrap_or(0);
    source[line..i].trim_start().starts_with("//")
}

fn replace_array_uses(source: &str, array: &HoistedArray) -> String {
    let name = array.name.as_str();
    let bytes = source.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(name.as_bytes())
            && (i == 0 || !is_ident_byte(bytes[i - 1]))
            && bytes.get(i + name.len()) == Some(&b'[')
        {
            let at = i + name.len();
            if let Some((idx1, next)) = parse_index(source, at) {
                if array.cols > 1 {
                    if let Some((idx2, next2)) = parse_index(source, next) {
                        let expr = format!(
                            "otdK({} + ({idx1}) * {} + ({idx2}))",
                            array.base, array.cols
                        );
                        push_use(&mut out, &expr, array.is_int);
                        i = next2;
                        continue;
                    }
                } else {
                    let expr = format!("otdK({} + ({idx1}))", array.base);
                    push_use(&mut out, &expr, array.is_int);
                    i = next;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn push_use(out: &mut String, expr: &str, is_int: bool) {
    if is_int {
        out.push_str("int(");
        out.push_str(expr);
        out.push(')');
    } else {
        out.push_str(expr);
    }
}

fn parse_index(source: &str, i: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(i) != Some(&b'[') {
        return None;
    }
    let mut square = 0i32;
    let mut paren = 0i32;
    let start = i + 1;
    let mut k = start;
    while k < bytes.len() {
        match bytes[k] {
            b'[' => square += 1,
            b']' if square == 0 && paren == 0 => {
                return Some((source[start..k].trim().to_string(), k + 1));
            }
            b']' => square -= 1,
            b'(' => paren += 1,
            b')' => paren -= 1,
            _ => {}
        }
        k += 1;
    }
    None
}

fn is_ident_byte(c: u8) -> bool {
    c == b'_' || c.is_ascii_alphanumeric()
}
