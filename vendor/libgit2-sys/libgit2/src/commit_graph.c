/*
 * Copyright (C) the libgit2 contributors. All rights reserved.
 *
 * This file is part of libgit2, distributed under the GNU GPL v2 with
 * a Linking Exception. For full terms see the included COPYING file.
 */

#include "commit_graph.h"

#include "futils.h"
#include "hash.h"
#include "pack.h"

#define GIT_COMMIT_GRAPH_MISSING_PARENT 0x70000000

#define COMMIT_GRAPH_SIGNATURE 0x43475048 /* "CGPH" */
#define COMMIT_GRAPH_VERSION 1
#define COMMIT_GRAPH_OBJECT_ID_VERSION 1
struct git_commit_graph_header {
	uint32_t signature;
	uint8_t version;
	uint8_t object_id_version;
	uint8_t chunks;
	uint8_t base_graph_files;
};

#define COMMIT_GRAPH_OID_FANOUT_ID 0x4f494446	      /* "OIDF" */
#define COMMIT_GRAPH_OID_LOOKUP_ID 0x4f49444c	      /* "OIDL" */
#define COMMIT_GRAPH_COMMIT_DATA_ID 0x43444154	      /* "CDAT" */
#define COMMIT_GRAPH_EXTRA_EDGE_LIST_ID 0x45444745    /* "EDGE" */
#define COMMIT_GRAPH_BLOOM_FILTER_INDEX_ID 0x42494458 /* "BIDX" */
#define COMMIT_GRAPH_BLOOM_FILTER_DATA_ID 0x42444154  /* "BDAT" */

struct git_commit_graph_chunk {
	off64_t offset;
	size_t length;
};

static int commit_graph_error(const char *message)
{
	git_error_set(GIT_ERROR_ODB, "invalid commit-graph file - %s", message);
	return -1;
}

static int commit_graph_parse_oid_fanout(
		git_commit_graph_file *cgraph,
		const unsigned char *data,
		struct git_commit_graph_chunk *chunk_oid_fanout)
{
	uint32_t i, nr;
	if (chunk_oid_fanout->offset == 0)
		return commit_graph_error("missing OID Fanout chunk");
	if (chunk_oid_fanout->length == 0)
		return commit_graph_error("empty OID Fanout chunk");
	if (chunk_oid_fanout->length != 256 * 4)
		return commit_graph_error("OID Fanout chunk has wrong length");

	cgraph->oid_fanout = (const uint32_t *)(data + chunk_oid_fanout->offset);
	nr = 0;
	for (i = 0; i < 256; ++i) {
		uint32_t n = ntohl(cgraph->oid_fanout[i]);
		if (n < nr)
			return commit_graph_error("index is non-monotonic");
		nr = n;
	}
	cgraph->num_commits = nr;
	return 0;
}

static int commit_graph_parse_oid_lookup(
		git_commit_graph_file *cgraph,
		const unsigned char *data,
		struct git_commit_graph_chunk *chunk_oid_lookup)
{
	uint32_t i;
	git_oid *oid, *prev_oid, zero_oid = {{0}};

	if (chunk_oid_lookup->offset == 0)
		return commit_graph_error("missing OID Lookup chunk");
	if (chunk_oid_lookup->length == 0)
		return commit_graph_error("empty OID Lookup chunk");
	if (chunk_oid_lookup->length != cgraph->num_commits * GIT_OID_RAWSZ)
		return commit_graph_error("OID Lookup chunk has wrong length");

	cgraph->oid_lookup = oid = (git_oid *)(data + chunk_oid_lookup->offset);
	prev_oid = &zero_oid;
	for (i = 0; i < cgraph->num_commits; ++i, ++oid) {
		if (git_oid_cmp(prev_oid, oid) >= 0)
			return commit_graph_error("OID Lookup index is non-monotonic");
		prev_oid = oid;
	}

	return 0;
}

static int commit_graph_parse_commit_data(
		git_commit_graph_file *cgraph,
		const unsigned char *data,
		struct git_commit_graph_chunk *chunk_commit_data)
{
	if (chunk_commit_data->offset == 0)
		return commit_graph_error("missing Commit Data chunk");
	if (chunk_commit_data->length == 0)
		return commit_graph_error("empty Commit Data chunk");
	if (chunk_commit_data->length != cgraph->num_commits * (GIT_OID_RAWSZ + 16))
		return commit_graph_error("Commit Data chunk has wrong length");

	cgraph->commit_data = data + chunk_commit_data->offset;

	return 0;
}

static int commit_graph_parse_extra_edge_list(
		git_commit_graph_file *cgraph,
		const unsigned char *data,
		struct git_commit_graph_chunk *chunk_extra_edge_list)
{
	if (chunk_extra_edge_list->length == 0)
		return 0;
	if (chunk_extra_edge_list->length % 4 != 0)
		return commit_graph_error("malformed Extra Edge List chunk");

	cgraph->extra_edge_list = data + chunk_extra_edge_list->offset;
	cgraph->num_extra_edge_list = chunk_extra_edge_list->length / 4;

	return 0;
}

int git_commit_graph_parse(git_commit_graph_file *cgraph, const unsigned char *data, size_t size)
{
	struct git_commit_graph_header *hdr;
	const unsigned char *chunk_hdr;
	struct git_commit_graph_chunk *last_chunk;
	uint32_t i;
	off64_t last_chunk_offset, chunk_offset, trailer_offset;
	git_oid cgraph_checksum = {{0}};
	int error;
	struct git_commit_graph_chunk chunk_oid_fanout = {0}, chunk_oid_lookup = {0},
				      chunk_commit_data = {0}, chunk_extra_edge_list = {0},
				      chunk_unsupported = {0};

	GIT_ASSERT_ARG(cgraph);

	if (size < sizeof(struct git_commit_graph_header) + GIT_OID_RAWSZ)
		return commit_graph_error("commit-graph is too short");

	hdr = ((struct git_commit_graph_header *)data);

	if (hdr->signature != htonl(COMMIT_GRAPH_SIGNATURE) || hdr->version != COMMIT_GRAPH_VERSION
	    || hdr->object_id_version != COMMIT_GRAPH_OBJECT_ID_VERSION) {
		return commit_graph_error("unsupported commit-graph version");
	}
	if (hdr->chunks == 0)
		return commit_graph_error("no chunks in commit-graph");

	/*
	 * The very first chunk's offset should be after the header, all the chunk
	 * headers, and a special zero chunk.
	 */
	last_chunk_offset = sizeof(struct git_commit_graph_header) + (1 + hdr->chunks) * 12;
	trailer_offset = size - GIT_OID_RAWSZ;
	if (trailer_offset < last_chunk_offset)
		return commit_graph_error("wrong commit-graph size");
	git_oid_cpy(&cgraph->checksum, (git_oid *)(data + trailer_offset));

	if (git_hash_buf(&cgraph_checksum, data, (size_t)trailer_offset) < 0)
		return commit_graph_error("could not calculate signature");
	if (!git_oid_equal(&cgraph_checksum, &cgraph->checksum))
		return commit_graph_error("index signature mismatch");

	chunk_hdr = data + sizeof(struct git_commit_graph_header);
	last_chunk = NULL;
	for (i = 0; i < hdr->chunks; ++i, chunk_hdr += 12) {
		chunk_offset = ((off64_t)ntohl(*((uint32_t *)(chunk_hdr + 4)))) << 32
				| ((off64_t)ntohl(*((uint32_t *)(chunk_hdr + 8))));
		if (chunk_offset < last_chunk_offset)
			return commit_graph_error("chunks are non-monotonic");
		if (chunk_offset >= trailer_offset)
			return commit_graph_error("chunks extend beyond the trailer");
		if (last_chunk != NULL)
			last_chunk->length = (size_t)(chunk_offset - last_chunk_offset);
		last_chunk_offset = chunk_offset;

		switch (ntohl(*((uint32_t *)(chunk_hdr + 0)))) {
		case COMMIT_GRAPH_OID_FANOUT_ID:
			chunk_oid_fanout.offset = last_chunk_offset;
			last_chunk = &chunk_oid_fanout;
			break;

		case COMMIT_GRAPH_OID_LOOKUP_ID:
			chunk_oid_lookup.offset = last_chunk_offset;
			last_chunk = &chunk_oid_lookup;
			break;

		case COMMIT_GRAPH_COMMIT_DATA_ID:
			chunk_commit_data.offset = last_chunk_offset;
			last_chunk = &chunk_commit_data;
			break;

		case COMMIT_GRAPH_EXTRA_EDGE_LIST_ID:
			chunk_extra_edge_list.offset = last_chunk_offset;
			last_chunk = &chunk_extra_edge_list;
			break;

		case COMMIT_GRAPH_BLOOM_FILTER_INDEX_ID:
		case COMMIT_GRAPH_BLOOM_FILTER_DATA_ID:
			chunk_unsupported.offset = last_chunk_offset;
			last_chunk = &chunk_unsupported;
			break;

		default:
			return commit_graph_error("unrecognized chunk ID");
		}
	}
	last_chunk->length = (size_t)(trailer_offset - last_chunk_offset);

	error = commit_graph_parse_oid_fanout(cgraph, data, &chunk_oid_fanout);
	if (error < 0)
		return error;
	error = commit_graph_parse_oid_lookup(cgraph, data, &chunk_oid_lookup);
	if (error < 0)
		return error;
	error = commit_graph_parse_commit_data(cgraph, data, &chunk_commit_data);
	if (error < 0)
		return error;
	error = commit_graph_parse_extra_edge_list(cgraph, data, &chunk_extra_edge_list);
	if (error < 0)
		return error;

	return 0;
}

int git_commit_graph_open(git_commit_graph_file **cgraph_out, const char *path)
{
	git_commit_graph_file *cgraph;
	git_file fd = -1;
	size_t cgraph_size;
	struct stat st;
	int error;

	/* TODO: properly open the file without access time using O_NOATIME */
	fd = git_futils_open_ro(path);
	if (fd < 0)
		return fd;

	if (p_fstat(fd, &st) < 0) {
		p_close(fd);
		git_error_set(GIT_ERROR_ODB, "multi-pack-index file not found - '%s'", path);
		return -1;
	}

	if (!S_ISREG(st.st_mode) || !git__is_sizet(st.st_size)) {
		p_close(fd);
		git_error_set(GIT_ERROR_ODB, "invalid pack index '%s'", path);
		return -1;
	}
	cgraph_size = (size_t)st.st_size;

	cgraph = git__calloc(1, sizeof(git_commit_graph_file));
	GIT_ERROR_CHECK_ALLOC(cgraph);

	error = git_buf_sets(&cgraph->filename, path);
	if (error < 0)
		return error;

	error = git_futils_mmap_ro(&cgraph->graph_map, fd, 0, cgraph_size);
	p_close(fd);
	if (error < 0) {
		git_commit_graph_free(cgraph);
		return error;
	}

	if ((error = git_commit_graph_parse(cgraph, cgraph->graph_map.data, cgraph_size)) < 0) {
		git_commit_graph_free(cgraph);
		return error;
	}

	*cgraph_out = cgraph;
	return 0;
}

static int git_commit_graph_entry_get_byindex(
		git_commit_graph_entry *e,
		const git_commit_graph_file *cgraph,
		size_t pos)
{
	const unsigned char *commit_data;

	GIT_ASSERT_ARG(e);
	GIT_ASSERT_ARG(cgraph);

	if (pos >= cgraph->num_commits) {
		git_error_set(GIT_ERROR_INVALID, "commit index %zu does not exist", pos);
		return GIT_ENOTFOUND;
	}

	commit_data = cgraph->commit_data + pos * (GIT_OID_RAWSZ + 4 * sizeof(uint32_t));
	git_oid_cpy(&e->tree_oid, (const git_oid *)commit_data);
	e->parent_indices[0] = ntohl(*((uint32_t *)(commit_data + GIT_OID_RAWSZ)));
	e->parent_indices[1]
			= ntohl(*((uint32_t *)(commit_data + GIT_OID_RAWSZ + sizeof(uint32_t))));
	e->parent_count = (e->parent_indices[0] != GIT_COMMIT_GRAPH_MISSING_PARENT)
			+ (e->parent_indices[1] != GIT_COMMIT_GRAPH_MISSING_PARENT);
	e->generation = ntohl(*((uint32_t *)(commit_data + GIT_OID_RAWSZ + 2 * sizeof(uint32_t))));
	e->commit_time = ntohl(*((uint32_t *)(commit_data + GIT_OID_RAWSZ + 3 * sizeof(uint32_t))));

	e->commit_time |= (e->generation & 0x3ull) << 32ull;
	e->generation >>= 2u;
	if (e->parent_indices[1] & 0x80000000u) {
		uint32_t extra_edge_list_pos = e->parent_indices[1] & 0x7fffffff;

		/* Make sure we're not being sent out of bounds */
		if (extra_edge_list_pos >= cgraph->num_extra_edge_list) {
			git_error_set(GIT_ERROR_INVALID,
				      "commit %u does not exist",
				      extra_edge_list_pos);
			return GIT_ENOTFOUND;
		}

		e->extra_parents_index = extra_edge_list_pos;
		while (extra_edge_list_pos < cgraph->num_extra_edge_list
		       && (ntohl(*(
					   (uint32_t *)(cgraph->extra_edge_list
							+ extra_edge_list_pos * sizeof(uint32_t))))
			   & 0x80000000u)
				       == 0) {
			extra_edge_list_pos++;
			e->parent_count++;
		}

	}
	git_oid_cpy(&e->sha1, &cgraph->oid_lookup[pos]);
	return 0;
}

bool git_commit_graph_needs_refresh(const git_commit_graph_file *cgraph, const char *path)
{
	git_file fd = -1;
	struct stat st;
	ssize_t bytes_read;
	git_oid cgraph_checksum = {{0}};

	if (path == NULL)
		path = git_buf_cstr(&cgraph->filename);

	/* TODO: properly open the file without access time using O_NOATIME */
	fd = git_futils_open_ro(path);
	if (fd < 0)
		return true;

	if (p_fstat(fd, &st) < 0) {
		p_close(fd);
		return true;
	}

	if (!S_ISREG(st.st_mode) || !git__is_sizet(st.st_size)
	    || (size_t)st.st_size != cgraph->graph_map.len) {
		p_close(fd);
		return true;
	}

	bytes_read = p_pread(fd, cgraph_checksum.id, GIT_OID_RAWSZ, st.st_size - GIT_OID_RAWSZ);
	p_close(fd);
	if (bytes_read != GIT_OID_RAWSZ)
		return true;

	return !git_oid_equal(&cgraph_checksum, &cgraph->checksum);
}

int git_commit_graph_entry_find(
		git_commit_graph_entry *e,
		const git_commit_graph_file *cgraph,
		const git_oid *short_oid,
		size_t len)
{
	int pos, found = 0;
	uint32_t hi, lo;
	const git_oid *current = NULL;

	GIT_ASSERT_ARG(e);
	GIT_ASSERT_ARG(cgraph);
	GIT_ASSERT_ARG(short_oid);

	hi = ntohl(cgraph->oid_fanout[(int)short_oid->id[0]]);
	lo = ((short_oid->id[0] == 0x0) ? 0 : ntohl(cgraph->oid_fanout[(int)short_oid->id[0] - 1]));

	pos = git_pack__lookup_sha1(cgraph->oid_lookup, GIT_OID_RAWSZ, lo, hi, short_oid->id);

	if (pos >= 0) {
		/* An object matching exactly the oid was found */
		found = 1;
		current = cgraph->oid_lookup + pos;
	} else {
		/* No object was found */
		/* pos refers to the object with the "closest" oid to short_oid */
		pos = -1 - pos;
		if (pos < (int)cgraph->num_commits) {
			current = cgraph->oid_lookup + pos;

			if (!git_oid_ncmp(short_oid, current, len))
				found = 1;
		}
	}

	if (found && len != GIT_OID_HEXSZ && pos + 1 < (int)cgraph->num_commits) {
		/* Check for ambiguousity */
		const git_oid *next = current + 1;

		if (!git_oid_ncmp(short_oid, next, len)) {
			found = 2;
		}
	}

	if (!found)
		return git_odb__error_notfound(
				"failed to find offset for multi-pack index entry", short_oid, len);
	if (found > 1)
		return git_odb__error_ambiguous(
				"found multiple offsets for multi-pack index entry");

	return git_commit_graph_entry_get_byindex(e, cgraph, pos);
}

int git_commit_graph_entry_parent(
		git_commit_graph_entry *parent,
		const git_commit_graph_file *cgraph,
		const git_commit_graph_entry *entry,
		size_t n)
{
	GIT_ASSERT_ARG(parent);
	GIT_ASSERT_ARG(cgraph);

	if (n >= entry->parent_count) {
		git_error_set(GIT_ERROR_INVALID, "parent index %zu does not exist", n);
		return GIT_ENOTFOUND;
	}

	if (n == 0 || (n == 1 && entry->parent_count == 2))
		return git_commit_graph_entry_get_byindex(parent, cgraph, entry->parent_indices[n]);

	return git_commit_graph_entry_get_byindex(
			parent,
			cgraph,
			ntohl(
					*(uint32_t *)(cgraph->extra_edge_list
						      + (entry->extra_parents_index + n - 1)
								      * sizeof(uint32_t)))
					& 0x7fffffff);
}


int git_commit_graph_close(git_commit_graph_file *cgraph)
{
	GIT_ASSERT_ARG(cgraph);

	if (cgraph->graph_map.data)
		git_futils_mmap_free(&cgraph->graph_map);

	return 0;
}

void git_commit_graph_free(git_commit_graph_file *cgraph)
{
	if (!cgraph)
		return;

	git_buf_dispose(&cgraph->filename);
	git_commit_graph_close(cgraph);
	git__free(cgraph);
}
