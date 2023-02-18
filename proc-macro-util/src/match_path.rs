use crate::*;

/// if subpath_candidate is a suffix contained in the provided full_path,
/// Some(syn::Path) is returned which is the unmatched portion of full_path
///
/// if subpath_candidate is empty an error will always be returned
///
/// # Example
/// ```
/// use proc_macro_util::{match_path, match_paths, UnmatchedPathPrefix};
/// use quote::quote;
///
/// assert_eq!(
///     syn::parse2::<syn::Path>(quote!(std::borrow)).unwrap(),
///     match_path(
///         &syn::parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
///         &syn::parse2(quote!(Borrow<Uuid>)).unwrap(),
///     ).unwrap().unwrap(),
/// );
///
/// assert!(match_path(
///     &syn::parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
///     &syn::parse2(quote!(Borrow<uuid::Uuid>)).unwrap(),
/// ).is_err());
///
/// assert!(match_path(
///     &syn::parse2(quote!(Borrow<Uuid>)).unwrap(),
///     &syn::parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
/// ).is_err());
///
/// assert_eq!(
///     syn::parse2::<syn::Path>(quote!(std::borrow)).unwrap(),
///     match_paths(
///         &[
///             syn::parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
///             syn::parse2(quote!(std::borrow::Borrow<uuid::Uuid>)).unwrap(),
///         ],
///         &syn::parse2(quote!(Borrow<Uuid>)).unwrap(),
///     ).unwrap().unwrap(),
/// );
///
/// assert_eq!(
///     syn::parse2::<syn::Path>(quote!(std::borrow)).unwrap(),
///     match_paths(
///         &[
///             syn::parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
///             syn::parse2(quote!(std::borrow::Borrow<uuid::Uuid>)).unwrap(),
///         ],
///         &syn::parse2(quote!(Borrow<uuid::Uuid>)).unwrap(),
///     ).unwrap().unwrap(),
/// );
///
/// assert!(
///     match_paths(
///         &[
///             syn::parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
///             syn::parse2(quote!(std::borrow::Borrow<uuid::Uuid>)).unwrap(),
///         ],
///         &syn::parse2(quote!(Borrow<Foo>)).unwrap(),
///     ).is_err(),
/// );
/// ```
pub fn match_path(full_path: &syn::Path, subpath_candidate: &syn::Path) -> Result<UnmatchedPathPrefix, MatchPathError> {
    let mut subpath_candidate_segments_iter = subpath_candidate.segments.iter();

    let mut subpath_candidate_segment = Some(
        subpath_candidate_segments_iter
            .next()
            .ok_or_else(|| MatchPathError::new(full_path, subpath_candidate, MatchIndex(None), NonMatchIndex(None)))?,
    );

    let mut first_match = Option::<usize>::None;

    for (full_path_index, segment) in full_path.segments.iter().enumerate() {
        let has_match_started = first_match.is_some();
        if subpath_candidate_segment.is_some() && *segment == *subpath_candidate_segment.unwrap() {
            if !has_match_started {
                first_match = Some(full_path_index);
            }
            subpath_candidate_segment = subpath_candidate_segments_iter.next();
        } else if has_match_started {
            return Err(MatchPathError::new(
                full_path,
                subpath_candidate,
                MatchIndex(first_match),
                NonMatchIndex(Some(full_path_index)),
            ));
        }
    }

    match first_match {
        Some(first_match) => Ok(get_unmatched_full_path_prefix(full_path, MatchIndex(Some(first_match)))),
        None => Err(MatchPathError::new(
            full_path,
            subpath_candidate,
            MatchIndex(None),
            NonMatchIndex(None),
        )),
    }
}

pub fn match_paths(full_paths: &[syn::Path], subpath_candidate: &syn::Path) -> Result<UnmatchedPathPrefix, Error> {
    for full_path in full_paths {
        if let Ok(unmatched_path_prefix) = match_path(full_path, subpath_candidate) {
            return Ok(unmatched_path_prefix);
        }
    }
    let supported_paths = full_paths
        .iter()
        .map(|path| format!("{}", quote!(#path)))
        .collect::<Vec<_>>()
        .join(", ");
    Err(Error::new_spanned(
        subpath_candidate,
        format!("path does not match the following supported values: {supported_paths}"),
    ))
}

/// newtype used to prevent accidental swapping of match and non-match indices
#[derive(Clone, Copy, Debug)]
struct MatchIndex(Option<usize>);

/// newtype used to prevent accidental swapping of match and non-match indices
#[derive(Clone, Copy, Debug)]
struct NonMatchIndex(Option<usize>);

#[derive(AsRef, AsMut, Clone, Deref, DerefMut, Derivative, Eq, Into, PartialEq)]
#[derivative(Debug)]
pub struct UnmatchedPathPrefix(#[derivative(Debug(format_with = "fmt_opt_syn_path"))] Option<syn::Path>);

impl UnmatchedPathPrefix {
    pub fn unwrap(self) -> syn::Path {
        self.0.unwrap()
    }
}

/// represents the diff between two paths which do not have a full suffix match
#[derive(Clone, Derivative, Eq, PartialEq)]
#[derivative(Debug)]
pub struct MatchPathError {
    #[derivative(Debug(format_with = "fmt_syn_path"))]
    pub full_path: syn::Path,
    #[derivative(Debug(format_with = "fmt_syn_path"))]
    pub subpath_candidate: syn::Path,
    /// the portion of the full_path which lies before the first matching PathSegment between the
    /// full_path and the subpath_candidate
    #[derivative(Debug(format_with = "fmt_opt_syn_path"))]
    pub unmatched_full_path_prefix: UnmatchedPathPrefix,
    /// the portion of the full_path which matches subpath_candidate PathSegments starting from
    /// the first matching PathSegment between the full_path and the subpath_candidate (None if
    /// no portions of the paths match)
    #[derivative(Debug(format_with = "fmt_opt_syn_path"))]
    pub matched: Option<syn::Path>,
    /// the portion of the full_path which lies after the matched portion (None if unmatched_full_path_prefix
    /// is the entirety of full_path, i.e. subpath_candidate trails on longer than the end of
    /// full_path)
    #[derivative(Debug(format_with = "fmt_opt_syn_path"))]
    pub unmatched_full_path_suffix: Option<syn::Path>,
    /// the portion of the subpath_candidate which lies after the matched portion if any (will
    /// contain the whole subpath_candidate path if no portion is matched from the full_path;
    /// is None if the subpath_candidate matches all the way through its segments but isn't long
    /// enough to cover the end of full_path)
    #[derivative(Debug(format_with = "fmt_opt_syn_path"))]
    pub unmatched_subpath_candidate_suffix: Option<syn::Path>,
}

impl MatchPathError {
    /// produces the different matching and non-matching segments of the input paths
    /// provided both the original full_path and subpath_candidate [syn::Path]s and the index in
    /// full_path of the first matching semgent (should be None if no match found) and the index
    /// in full_path of the first non-matching segment (should only be Some if a matching segment
    /// was found and subsequently a non-matching segement was found)
    fn new(
        full_path: &syn::Path,
        subpath_candidate: &syn::Path,
        first_match: MatchIndex,
        first_nonmatch: NonMatchIndex,
    ) -> Self {
        Self {
            full_path: full_path.clone(),
            subpath_candidate: subpath_candidate.clone(),
            unmatched_full_path_prefix: get_unmatched_full_path_prefix(full_path, first_match),
            matched: get_matched(full_path, first_match, first_nonmatch),
            unmatched_full_path_suffix: get_unmatched_full_path_suffix(full_path, first_nonmatch),
            unmatched_subpath_candidate_suffix: get_unmatched_subpath_candidate_suffix(
                full_path,
                subpath_candidate,
                first_match,
                first_nonmatch,
            ),
        }
    }
}

impl From<MatchPathError> for Error {
    fn from(value: MatchPathError) -> Self {
        let MatchPathError {
            full_path,
            subpath_candidate,
            ..
        } = value;
        Error::new_spanned(
            subpath_candidate,
            format!("path does not match the supported full path: {}", quote!(#full_path)),
        )
    }
}

fn fmt_syn_path(path: &syn::Path, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    write!(f, "path=`{}`", quote!(#path))
}

fn fmt_opt_syn_path(path: &Option<syn::Path>, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    match path.as_ref() {
        Some(path) => fmt_syn_path(path, f),
        None => write!(f, "None"),
    }
}

fn get_unmatched_full_path_prefix(full_path: &syn::Path, first_match: MatchIndex) -> UnmatchedPathPrefix {
    let first_match = match first_match.0 {
        Some(first_match) => first_match,
        None => return UnmatchedPathPrefix(Some(full_path.clone())),
    };
    if first_match == 0 {
        return UnmatchedPathPrefix(None);
    }
    UnmatchedPathPrefix(Some(syn::Path {
        leading_colon: full_path.leading_colon,
        segments: Punctuated::from_iter(full_path.segments.iter().map(Clone::clone).take(first_match)),
    }))
}

fn get_matched(full_path: &syn::Path, first_match: MatchIndex, first_nonmatch: NonMatchIndex) -> Option<syn::Path> {
    let match_len = get_match_len(full_path, first_match, first_nonmatch)?;
    Some(syn::Path {
        leading_colon: first_match.0.and(full_path.leading_colon),
        segments: Punctuated::from_iter(
            full_path
                .segments
                .iter()
                .map(Clone::clone)
                .skip(first_match.0.unwrap())
                .take(match_len),
        ),
    })
}

fn get_unmatched_full_path_suffix(full_path: &syn::Path, first_nonmatch: NonMatchIndex) -> Option<syn::Path> {
    let first_nonmatch = first_nonmatch.0?;
    if first_nonmatch == full_path.segments.len() {
        return None;
    }
    Some(syn::Path {
        leading_colon: None,
        segments: Punctuated::from_iter(full_path.segments.iter().map(Clone::clone).skip(first_nonmatch)),
    })
}

fn get_unmatched_subpath_candidate_suffix(
    full_path: &syn::Path,
    subpath_candidate: &syn::Path,
    first_match: MatchIndex,
    first_nonmatch: NonMatchIndex,
) -> Option<syn::Path> {
    let match_len = get_match_len(full_path, first_match, first_nonmatch)?;
    Some(syn::Path {
        leading_colon: None,
        segments: Punctuated::from_iter(
            subpath_candidate
                .segments
                .iter()
                .map(Clone::clone)
                .skip(first_match.0.unwrap() + match_len),
        ),
    })
}

fn get_match_len(full_path: &syn::Path, first_match: MatchIndex, first_nonmatch: NonMatchIndex) -> Option<usize> {
    let first_match = first_match.0?;
    Some(match first_nonmatch.0 {
        Some(first_nonmatch) => first_nonmatch - first_match,
        None => full_path.segments.len() - first_match,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse2;

    #[derive(Clone, Debug)]
    struct TestCase {
        expected: Result<UnmatchedPathPrefix, MatchPathError>,
        full_path: syn::Path,
        subpath_candidate: syn::Path,
    }

    fn test_cases() -> Vec<TestCase> {
        vec![TestCase {
            expected: Ok(UnmatchedPathPrefix(None)),
            full_path: parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
            subpath_candidate: parse2(quote!(std::borrow::Borrow<Uuid>)).unwrap(),
        }]
    }

    #[test]
    fn test_match_path_test_cases() {
        for (i, test_case) in test_cases().into_iter().enumerate() {
            let result = match_path(&test_case.full_path, &test_case.subpath_candidate);
            if test_case.expected != result {
                println!("{:?}", UnmatchedPathPrefix(None));
                panic!(
                    "test case {i} failed: expected {:?}, received {result:?}",
                    test_case.expected
                );
            }
        }
    }
}
