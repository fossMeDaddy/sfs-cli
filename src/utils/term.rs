pub fn get_formatted_cols<'a, I, S>(data: I, n_cols: usize) -> String
where
    S: AsRef<str>,
    I: ExactSizeIterator<Item = S> + Clone,
{
    let mut n_col_padding: Vec<u32> = vec![0; n_cols];

    let mut i = 0;
    while i < data.len() {
        for j in 0..n_cols {
            let path_str = match data.clone().nth(i + j) {
                Some(p) => p,
                None => continue,
            };
            let path_str = path_str.as_ref();

            n_col_padding[j] = n_col_padding[j].max(path_str.len() as u32);
        }

        i += n_cols;
    }

    let mut output_str = String::new();
    let mut i = 0;
    while i < data.len() {
        for j in 0..n_cols {
            let data_str = match data.clone().nth(i + j) {
                Some(p) => p,
                None => continue,
            };
            let data_str = data_str.as_ref();

            output_str += &format!("{0:<1$}  ", data_str, n_col_padding[j] as usize);
        }
        if i + n_cols < data.len() - 1 {
            output_str += "\n";
        }

        i += n_cols;
    }

    output_str
}
