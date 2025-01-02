pub fn decode_access_token(token_str: &str) -> String {
    token_str.replace("-", "+").replace("~", "/")
}
