use std::collections::BTreeSet;
use std::path::Path;

pub async fn resolve_patterns(patterns: &[String]) -> anyhow::Result<Vec<String>> {
    let cwd = std::env::current_dir()?;
    let mut all_files = BTreeSet::new();

    for pattern in patterns {
        let files = resolve_pattern(pattern, &cwd).await?;
        all_files.extend(files);
    }

    Ok(all_files.into_iter().collect())
}

async fn resolve_pattern(pattern: &str, cwd: &Path) -> anyhow::Result<Vec<String>> {
    let resolved = cwd.join(pattern);

    if let Ok(metadata) = tokio::fs::metadata(&resolved).await {
        if metadata.is_file() && pattern.ends_with(".hone") {
            return Ok(vec![resolved.to_string_lossy().into_owned()]);
        }

        if metadata.is_dir() {
            let glob_pattern = format!("{}/**/*.hone", resolved.to_string_lossy());
            let paths = glob::glob(&glob_pattern)?;
            let results: Vec<String> = paths
                .filter_map(Result::ok)
                .filter(|p| p.is_file())
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            return Ok(results);
        }
    }

    let glob_pattern = if Path::new(pattern).is_absolute() {
        pattern.to_string()
    } else {
        cwd.join(pattern).to_string_lossy().into_owned()
    };

    let paths = glob::glob(&glob_pattern)?;
    let results: Vec<String> = paths
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    Ok(results)
}
