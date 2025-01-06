pub fn get_formatted_cols<'a, I, S>(data: I, n_cols: usize) -> String
where
    S: AsRef<str>,
    I: ExactSizeIterator<Item = S> + Clone,
{
    let mut n_col_padding: Vec<u32> = vec![0; n_cols];

    let mut padding_iter = data.clone().peekable();
    'outer: while padding_iter.peek().is_some() {
        for j in 0..n_cols {
            let path_str = match padding_iter.next() {
                Some(p) => p,
                None => {
                    break 'outer;
                }
            };
            let path_str = path_str.as_ref();

            n_col_padding[j] = n_col_padding[j].max((path_str.len() + 2) as u32);
        }
    }

    let mut output_str = String::new();
    let mut output_iter = data.clone().peekable();
    'outer: while output_iter.peek().is_some() {
        for j in 0..n_cols {
            let data_str = match output_iter.next() {
                Some(p) => p,
                None => break 'outer,
            };
            let data_str = data_str.as_ref();

            output_str += &format!("{0:<1$}  ", data_str, n_col_padding[j] as usize);
        }

        output_str += "\n";
    }

    output_str
}
