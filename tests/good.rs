use xmltree::Document;

const SRC: &str = include_str!("good.xml");
const CMP: &str = include_str!("good.xml.parsed");

#[test]
fn test_good() {
    let document = match Document::parse_str(SRC) {
        Ok(doc) => doc,
        Err(e) => panic!("{e}"),
    };

    let str = document.to_xml(None).unwrap();
    //    std::fs::write("tests/good.xml.parsed", &str).unwrap();
    if str != CMP {
        let mut src_lines = str.lines();
        for (i, line) in CMP.lines().enumerate() {
            match src_lines.next() {
                None => {
                    panic!(
                        "Diff at line {}: Not enough rows in parsed document!",
                        i + 1
                    );
                }

                Some(l) => {
                    if line == l {
                        continue;
                    } else {
                        eprintln!("Diff at line {}", i + 1);

                        eprintln!("Expected: {line}");
                        eprintln!("Got: {l}");
                        break;
                    }
                }
            }
        }

        panic!("Parsed document does not match expected output");
    }
}

#[test]
fn test_encode() {
    let document = match Document::parse_str(SRC) {
        Ok(doc) => doc,
        Err(e) => panic!("{e}"),
    };

    let bytes = document.to_bin().unwrap();
    let document2 = Document::from_bin(&bytes).expect("Could not decode document");

    assert_eq!(document, document2);
}
