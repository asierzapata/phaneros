use phaneros::scanner::Scanner;

fn main() {
    let mut scanner = Scanner::new(String::from("/Users/asierzapata/Documents/"));

    let scanner_events = scanner.events();

    scanner_events.subscribe(phaneros::scanner::ScannerEvent::ScanStarted, |file_path| {
        println!("Scan started for path: {}", file_path);
    });

    scanner_events.subscribe(
        phaneros::scanner::ScannerEvent::ScanCompleted,
        |file_path| {
            println!("Scan completed for path: {}", file_path);
        },
    );

    let scanner_results = scanner.scan();

    println!("Scanner results: {:?}", scanner_results);
}
