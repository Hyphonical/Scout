# Quick Reference: Export & Path Functions

This is a quick reference guide for Scout's export and path output functionality.

## 🔍 Search Export & Paths

### Basic Export

```bash
# Export search results to JSON file
scout search "mountains" --export results.json

# Export to stdout
scout search "sunset" --export -

# Output only file paths (no JSON)
scout search "cat" --paths
```

### Cross-Platform File Operations

#### Windows (PowerShell)

```powershell
# Copy search results to backup folder
scout search "vacation 2024" --paths | ForEach-Object { Copy-Item $_ "C:\Backup\" }

# Move low-quality images to review folder
scout search "blurry" --paths | ForEach-Object { Move-Item $_ "C:\ToReview\" }

# Create hardlinks for results
scout search "favorites" --paths | ForEach-Object { New-Item -ItemType HardLink -Path "C:\Favorites\$($_ | Split-Path -Leaf)" -Target $_ }

# Process with JSON
scout search "portrait" --export results.json
$data = Get-Content results.json | ConvertFrom-Json
foreach ($result in $data.results | Where-Object { $_.score -gt 0.7 }) {
    Copy-Item $result.path "C:\HighQuality\"
}

# Count results
(scout search "landscape" --paths | Measure-Object -Line).Lines
```

#### Linux / macOS (Bash)

```bash
# Copy search results to backup folder
scout search "vacation 2024" --paths | xargs -I {} cp {} /backup/vacation/

# Move files to review folder
scout search "blurry" --paths | xargs -I {} mv {} ./to_review/

# Create symbolic links
scout search "favorites" --paths | xargs -I {} ln -s {} ~/Favorites/

# Process with jq (filter by score)
scout search "portrait" --export - | jq -r '.results[] | select(.score > 0.7) | .path' | xargs -I {} cp {} ./high_quality/

# Count results
scout search "landscape" --paths | wc -l

# Archive results to tar
scout search "project photos" --paths | tar -czf project_photos.tar.gz -T -

# Generate file list for rsync
scout search "important" --paths > files_to_backup.txt
rsync -av --files-from=files_to_backup.txt / /backup/
```

## 🗂️ Cluster Export & Organization

### Basic Cluster Export

```bash
# Export clusters to JSON file
scout cluster --export clusters.json

# Export to stdout
scout cluster --export -
```

### Organize Clusters into Folders

#### Windows (PowerShell)

```powershell
# Organize files into cluster folders
scout cluster --export clusters.json
$data = Get-Content clusters.json | ConvertFrom-Json

foreach ($cluster in $data.clusters) {
    # Create folder for cluster
    New-Item -ItemType Directory -Force -Path "Cluster_$($cluster.id)"
    
    # Copy all members to cluster folder
    foreach ($file in $cluster.members) {
        $filename = Split-Path $file -Leaf
        Copy-Item $file "Cluster_$($cluster.id)\$filename"
    }
}

# Copy only high-cohesion clusters
$data.clusters | Where-Object { $_.cohesion -gt 0.8 } | ForEach-Object {
    New-Item -ItemType Directory -Force -Path "HighCohesion_$($_.id)"
    $_.members | ForEach-Object {
        Copy-Item $_ "HighCohesion_$($_.id)\"
    }
}

# Copy representative files to showcase
New-Item -ItemType Directory -Force -Path "Representatives"
$data.clusters | ForEach-Object {
    Copy-Item $_.representative "Representatives\"
}
```

#### Linux / macOS (Bash with jq)

```bash
# Organize files into cluster folders
scout cluster --export - | jq -r '.clusters[] | @json' | while read cluster; do
    id=$(echo $cluster | jq -r '.id')
    mkdir -p "cluster_$id"
    echo $cluster | jq -r '.members[]' | xargs -I {} cp {} "cluster_$id/"
done

# Alternative: simpler one-liner per cluster
scout cluster --export clusters.json
cat clusters.json | jq -r '.clusters[] | "mkdir -p cluster_\(.id) && cp \(.members | join(" ")) cluster_\(.id)/"' | sh

# Copy only high-cohesion clusters (>80%)
scout cluster --export - | jq -r '.clusters[] | select(.cohesion > 0.8) | "mkdir -p high_cohesion_\(.id) && echo \(.members[]) | xargs -I {} cp {} high_cohesion_\(.id)/"' | sh

# Copy representative files to showcase folder
mkdir -p showcase
scout cluster --export - | jq -r '.clusters[].representative' | xargs -I {} cp {} showcase/

# Create symbolic links instead of copies
scout cluster --export - | jq -r '.clusters[] | @json' | while read cluster; do
    id=$(echo $cluster | jq -r '.id')
    mkdir -p "cluster_$id"
    echo $cluster | jq -r '.members[]' | xargs -I {} ln -s $(realpath {}) "cluster_$id/"
done
```

## 📊 Advanced jq Queries

### Analyzing Search Results

```bash
# Get all file paths
scout search "sunset" --export - | jq -r '.results[].path'

# Get paths with score > 0.5
scout search "sunset" --export - | jq -r '.results[] | select(.score > 0.5) | .path'

# Get top 5 results
scout search "sunset" --export - | jq -r '.results[:5][].path'

# Create CSV of results
scout search "landscape" --export - | jq -r '.results[] | [.path, .score] | @csv'

# Average score of results
scout search "portrait" --export - | jq '[.results[].score] | add / length'

# Count results by score range
scout search "cat" --export - | jq '[.results[].score] | group_by(. >= 0.5) | map({range: (if .[0] >= 0.5 then "high" else "low" end), count: length})'

# Find video results (have timestamp field)
scout search "action" --export - | jq -r '.results[] | select(.timestamp != null) | .path'
```

### Analyzing Clusters

```bash
# Count files per cluster
scout cluster --export - | jq '.clusters[] | {id, count: .members | length}'

# Find largest cluster
scout cluster --export - | jq '.clusters | max_by(.size)'

# Find clusters with high cohesion (>85%)
scout cluster --export - | jq '.clusters[] | select(.cohesion > 0.85) | {id, cohesion, size}'

# Get all noise files
scout cluster --export - | jq -r '.noise[]'

# Count total noise
scout cluster --export - | jq '.noise | length'

# Average cohesion across all clusters
scout cluster --export - | jq '[.clusters[].cohesion] | add / length'

# Get members of specific cluster (e.g., cluster 3)
scout cluster --export - | jq -r '.clusters[] | select(.id == 3) | .members[]'

# Find cluster containing specific file
scout cluster --export - | jq '.clusters[] | select(.members[] | contains("sunset.jpg")) | .id'

# Summary statistics
scout cluster --export - | jq '{
    total_clusters: .clusters | length,
    total_files: .total_images,
    noise_count: .noise | length,
    avg_cluster_size: ([.clusters[].size] | add / length),
    avg_cohesion: ([.clusters[].cohesion] | add / length)
}'
```

## 🔄 Common Workflows

### Workflow 1: Export Similar Images to New Project

```bash
# Search for project-related images and copy to new folder
mkdir ~/NewProject
scout search "product photos 2024" --paths | xargs -I {} cp {} ~/NewProject/

# Or with filtering by score (macOS/Linux)
scout search "product photos 2024" --export - | \
    jq -r '.results[] | select(.score > 0.6) | .path' | \
    xargs -I {} cp {} ~/NewProject/
```

### Workflow 2: Organize by Clusters

```bash
# Cluster images and organize into folders
cd ~/Photos
scout cluster --export - | jq -r '.clusters[] | @json' | while read cluster; do
    id=$(echo $cluster | jq -r '.id')
    cohesion=$(echo $cluster | jq -r '.cohesion')
    mkdir -p "organized/cluster_${id}_${cohesion}"
    echo $cluster | jq -r '.members[]' | xargs -I {} cp {} "organized/cluster_${id}_${cohesion}/"
done
```

### Workflow 3: Backup High-Quality Images

```bash
# Find high-quality images and backup
scout search "sharp clear" --export - | \
    jq -r '.results[] | select(.score > 0.8) | .path' | \
    rsync -av --files-from=- / /backup/high_quality/
```

### Workflow 4: Find and Remove Duplicates

```bash
# Find near-duplicates (use reference image)
scout search -i original.jpg -s 0.9 --paths | tail -n +2 > duplicates.txt

# Review duplicates
cat duplicates.txt

# Move to duplicates folder
mkdir duplicates
cat duplicates.txt | xargs -I {} mv {} duplicates/
```

### Workflow 5: Create Playlist from Video Search

```bash
# Find videos and create M3U playlist
echo "#EXTM3U" > playlist.m3u
scout search "concert footage" --export - | \
    jq -r '.results[] | select(.timestamp != null) | .path' >> playlist.m3u
```

### Workflow 6: Generate HTML Gallery

```bash
# Create simple HTML gallery from search results
cat > gallery.html <<'EOF'
<!DOCTYPE html>
<html><head><title>Gallery</title></head><body>
EOF

scout search "vacation 2023" --export - | \
    jq -r '.results[] | "<img src=\"\(.path)\" style=\"max-width:300px;\"><br>"' >> gallery.html

echo "</body></html>" >> gallery.html
```

## 🛠️ Troubleshooting

### Path Issues

```bash
# If paths have spaces, use proper quoting
scout search "test" --paths | while IFS= read -r file; do
    cp "$file" /destination/
done

# For Windows paths in WSL
scout search "test" --paths | sed 's/\\/\//g' | sed 's/C:/\/mnt\/c/'
```

### Large Result Sets

```bash
# Process in batches to avoid argument list too long
scout search "all photos" --paths | xargs -n 100 -I {} cp {} /destination/

# Or use find with exec
scout search "all photos" --paths | xargs -I {} find {} -exec cp {} /destination/ \;
```

### JSON Parsing Errors

```bash
# Validate JSON first
scout search "test" --export - | jq empty

# Pretty-print for debugging
scout search "test" --export - | jq '.'

# Check for specific field
scout search "test" --export - | jq 'has("results")'
```

## 📋 Tips & Tricks

1. **Use `--export -` for piping**: Faster than writing to file first
2. **Combine with grep**: `scout search "cat" --paths | grep -i "2024"`
3. **Use jq's `-r` flag**: Gets raw strings without quotes
4. **Test with `--limit 5` first**: Avoid processing thousands of files accidentally
5. **Dry run with echo**: `scout search "test" --paths | xargs -I {} echo cp {} /dest/`
6. **Use absolute paths**: More reliable than relative paths in scripts
7. **Check exit codes**: `scout search "test" --paths > /dev/null && echo "Found results"`
8. **Combine multiple searches**: `scout search "cat" --paths && scout search "dog" --paths`

## 🔗 Related Commands

- `jq` - JSON processor ([stedolan.github.io/jq](https://stedolan.github.io/jq/))
- `xargs` - Build and execute command lines
- `rsync` - Remote file synchronization
- `find` - Search for files
- `grep` - Pattern matching
- `sed` - Stream editor
- `awk` - Pattern scanning and processing

## 📚 Further Reading

- [README.md](README.md) - Main documentation
- [docs/USER-GUIDE.md](docs/USER-GUIDE.md) - Complete user guide
- [SUGGESTIONS.md](SUGGESTIONS.md) - Feature suggestions and improvements
- [jq Manual](https://stedolan.github.io/jq/manual/) - Complete jq reference
