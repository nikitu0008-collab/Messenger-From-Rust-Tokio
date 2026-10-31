mod ChatRoom;
mod Buffer;
mod HandleClient;
mod HTML;
mod URL_DECODE;

fn main() {
    let html_lines: Vec<String> = HTML::getHTMLVector();
    
    let full_html: String = html_lines.join("\n");

    println!("HTML Code:\n{}", full_html);
}
