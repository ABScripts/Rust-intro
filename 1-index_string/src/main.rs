use std::collections::HashMap;

#[test]
fn test_word_positions_for_simple_string() {
    const NAIVE_TEXT: &str = "it will be great if it works";
    let mut expected_word_positions = HashMap::<&str, Vec<usize>>::new();
    expected_word_positions.insert("it", vec![0, 20]);
    expected_word_positions.insert("be", vec![8]);
    expected_word_positions.insert("great", vec![11]);
    expected_word_positions.insert("if", vec![17]);
    expected_word_positions.insert("will", vec![3]);
    expected_word_positions.insert("works", vec![23]);

    assert_eq!(get_word_positions(NAIVE_TEXT), expected_word_positions);
}

#[test]
fn test_word_positions_for_complex_string() {
    const COMPLEX_TEXT: &str = "    ,.,  it   will be great if,,, it works    ,.,.";
    let mut expected_word_positions = HashMap::<&str, Vec<usize>>::new();
    expected_word_positions.insert("it", vec![9, 34]);
    expected_word_positions.insert("be", vec![19]);
    expected_word_positions.insert("great", vec![22]);
    expected_word_positions.insert("if", vec![28]);
    expected_word_positions.insert("will", vec![14]);
    expected_word_positions.insert("works", vec![37]);

    assert_eq!(get_word_positions(COMPLEX_TEXT), expected_word_positions);
}

#[test]
fn test_word_positions_for_emtpy_string() {
    assert_eq!(get_word_positions(""), HashMap::<&str, Vec<usize>>::new());
}

#[test]
fn test_word_positions_for_string_with_just_punctuation_symbols() {
    assert_eq!(
        get_word_positions("     ,.       ,  . , , , "),
        HashMap::<&str, Vec<usize>>::new()
    );
}

fn get_word_positions(string_to_index: &str) -> HashMap<&str, Vec<usize>> {
    let mut indexed_string = HashMap::<&str, Vec<usize>>::new();

    let mut is_tracking_word = false;
    let mut word_start_index = 0;
    for (index, ch) in string_to_index.chars().enumerate() {
        let is_ch_punctuation = ch.is_ascii_punctuation() || ch.is_ascii_whitespace();

        if is_tracking_word {
            if is_ch_punctuation {
                let found_word = &string_to_index[word_start_index..index];
                indexed_string
                    .entry(found_word)
                    .or_default()
                    .push(word_start_index);

                is_tracking_word = false;
            }
        } else {
            if !is_ch_punctuation {
                is_tracking_word = true;
                word_start_index = index
            }
        }
    }

    if is_tracking_word {
        let found_word = &string_to_index[word_start_index..];
        indexed_string
            .entry(found_word)
            .or_default()
            .push(word_start_index);
    }

    indexed_string
}

fn main() {
    const LARGE_TEXT : &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam aliquam,
        ligula id malesuada ultrices, urna sem convallis diam, et blandit metus lectus id est. Phasellus
        pulvinar magna a justo tristique, quis faucibus eros elementum. Suspendisse aliquet et eros eu consectetur.
        In laoreet massa at scelerisque ultricies. Pellentesque in fringilla diam, sed vestibulum ligula. Vestibulum
        a lorem sodales felis maximus dignissim ac non turpis. Curabitur turpis quam, tempus eget malesuada sed, elementum
        at risus. Vivamus at placerat sapien, nec vulputate justo. Nam commodo lorem nibh. Duis vehicula justo non ullamcorper ornare.
        Praesent maximus sem nec dolor varius euismod. Nullam interdum mattis risus, vitae condimentum libero.";

    let word_positions = get_word_positions(LARGE_TEXT);
    println!("{word_positions:?}");
}
