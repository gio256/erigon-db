use crate::kv::EnvFlags;

// https://github.com/ledgerwatch/erigon-lib/blob/625c9f5385d209dc2abfadedf6e4b3914a26ed3e/kv/mdbx/kv_mdbx.go#L154
const ENV_FLAGS: EnvFlags = EnvFlags {
    // Disable readahead. Improves performance when db size > RAM.
    no_rdahead: true,
    // Try to coalesce while garbage collecting. (https://en.wikipedia.org/wiki/Coalescing_(computer_science))
    coalesce: true,
    // If another process is using the db with different flags, open in
    // compatibility mode instead of MDBX_INCOMPATIBLE error.
    accede: true,
    no_sub_dir: false,
    exclusive: false,
    no_meminit: false,
    liforeclaim: false,
};
