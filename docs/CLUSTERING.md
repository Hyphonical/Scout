# Clustering in Scout

Scout uses advanced density-based clustering to automatically group your images and videos by visual similarity. Unlike traditional methods like K-Means, Scout doesn't require you to specify the number of clusters beforehand and can identify "noise" (outliers) that doesn't belong anywhere.

## The Clustering Pipeline

The clustering process follows a sophisticated three-stage pipeline:

1. **Feature Extraction**: High-dimensional embeddings are retrieved from sidecar metadata.
2. **Dimension Reduction (UMAP)**: Optional projection of high-dimensional space into a lower-dimensional manifold.
3. **Density Clustering (HDBSCAN)**: Discovery of variable-density clusters and noise extraction.
4. **Metric Computation**: Calculation of cluster cohesion and selection of representative "key" images.

---

## 1. Dimensionality Reduction with UMAP

Scout uses **UMAP** (Uniform Manifold Approximation and Projection) for dimensionality reduction. When dealing with the 1024-dimensional embeddings from SigLIP2, clustering can be computationally expensive and suffer from the "curse of dimensionality."

### The Math behind UMAP
UMAP is based on manifold learning and algebraic topology. It assumes that the data is uniformly distributed on a local Riemannian manifold. It works in two steps:

1.  **Graph Construction**: It constructs a fuzzy simplicial complex by finding $k$-nearest neighbors. The probability of an edge between points $i$ and $j$ is:
    $$p_{j|i} = \exp\left(-\frac{d(x_i, x_j) - \rho_i}{\sigma_i}\right)$$
    where $\rho_i$ is the distance to the nearest neighbor and $\sigma_i$ is the local connectivity parameter.

2.  **Optimization**: It initializes a low-dimensional representation and optimizes it to minimize the cross-entropy between the high-dimensional graph ($P$) and low-dimensional graph ($Q$):
    $$C = \sum_{i \neq j} \left[ p_{ij} \log\left(\frac{p_{ij}}{q_{ij}}\right) + (1 - p_{ij}) \log\left(\frac{1 - p_{ij}}{1 - q_{ij}}\right) \right]$$

In Scout, UMAP reduces the **1024D** embeddings to **512D** (or lower) to speed up HDBSCAN while preserving the local and global structure of your image library.

---

## 2. Clustering with HDBSCAN

For the actual grouping, Scout employs **HDBSCAN** (Hierarchical Density-Based Spatial Clustering of Applications with Noise).

### Why HDBSCAN?
- **No $K$ required**: It automatically determines the number of clusters.
- **Variable Density**: It finds clusters that have different densities, which is common in photo collections (e.g., hundreds of "beach" photos vs. five "sunset" photos).
- **Noise Awareness**: It marks outliers as "noise" (Label -1) instead of forcing them into a cluster.

### The Algorithm
HDBSCAN transforms the space using the **Mutual Reachability Distance**:
$$d_{mreach-k}(a, b) = \max\{core_k(a), core_k(b), d(a, b)\}$$
where $core_k(x)$ is the distance to the $k$-th nearest neighbor. This "spreads out" low-density points and effectively pushes noise away from clusters.

It then builds a **Minimum Spanning Tree (MST)** and constructs a hierarchy of clusters. Finally, it uses **Cluster Stability** to extract the most persistent clusters from the tree.

---

## 3. Cluster Metrics

Once clusters are formed, Scout performs post-processing to help you understand what's in them.

### Representative Selection
Scout finds the "archetype" for each cluster—the image that best represents the group. It calculates the **Centroid** (mean vector) in the original 1024D space:
$$\mu = \frac{1}{N} \sum_{i=1}^{N} E_i$$
The image whose embedding has the highest **Cosine Similarity** to $\mu$ is selected as the representative:
$$\text{Representative} = \arg\max_{i} \left( \frac{E_i \cdot \mu}{\|E_i\| \|\mu\|} \right)$$

### Cohesion Score
To tell you how "tight" a cluster is, Scout calculates a **Cohesion Score** (0-100%). This is the average pairwise similarity between all images in the cluster:
$$\text{Cohesion} = \frac{2}{N(N-1)} \sum_{i < j} \text{sim}(E_i, E_j)$$
A cluster of identical photos will have ~100% cohesion, while a loose group of "nature" photos might have 70-80%.

---

## Summary of Parameters

- `min-cluster-size`: The minimum number of images needed to form a group (default: 5).
- `min-samples`: Adjusts how conservative the clustering is. Higher values result in more noise and fewer, more "core" clusters.
- `use-umap`: Enables UMAP dimension reduction. Recommended for collections larger than 1,000 images.
