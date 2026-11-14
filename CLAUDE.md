# weft

Side project: "JAX for general SWE". Logic is written in a restricted model with
explicit escape hatches. A compiler handles distribution, coordination, storage
and testing. The running example is a simple CRUD app.

The first scaffold (Python tracing front end, IR v0, deterministic simulator) is
in commit fb58534, kept for reference. The redesign moves the user-facing layer
up to a logical program plus placement/consistency annotations.

## Reference code

`refs/` holds read-only clones of related projects (Hydro, JAX, Triton, Wing,
Encore, DBOS, TigerBeetle, Maelstrom, Roc, Koka). [refs/README.md](refs/README.md) says why
each matters and where to start reading.

- Never edit files under `refs/`; `refs/sync.sh` resets them.
- They're git-ignored, so project-wide searches skip them. Search them by path,
  e.g. `rg pattern refs/jax`.
- Add a repo by editing `refs/repos.txt` and running `refs/sync.sh <name>`.
- Restate (BSL) and Convex (FSL) are source-available: read, don't copy.
