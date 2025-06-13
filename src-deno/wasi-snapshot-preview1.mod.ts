import Context from './wasi-snapshot-preview1.ts'

/** @type {{ [index: string]: string }} */
let env = {};
try {
    env = Deno.env.toObject();
} catch (error) {
    console.error(`Failed to get environment variables so running without them: `, error);
}

const context = new Context({
    args: Deno.args,
    env,
});

// TODO: how do we access the WebAssembly module's memory here?
//       investigate how "npm:@bytecodealliance/preview2-shim@^0.17.1" does it, search for "cabiLowerSymbol"

export function args_get(...args: unknown[]) { return (context.exports.args_get as (...args: unknown[]) => void)(...args); }
export function args_sizes_get(...args: unknown[]) { return (context.exports.args_sizes_get as (...args: unknown[]) => void)(...args); }
export function environ_get(...args: unknown[]) { return (context.exports.environ_get as (...args: unknown[]) => void)(...args); }
export function environ_sizes_get(...args: unknown[]) { return (context.exports.environ_sizes_get as (...args: unknown[]) => void)(...args); }
export function clock_res_get(...args: unknown[]) { return (context.exports.clock_res_get as (...args: unknown[]) => void)(...args); }
export function clock_time_get(...args: unknown[]) { return (context.exports.clock_time_get as (...args: unknown[]) => void)(...args); }
export function fd_advise(...args: unknown[]) { return (context.exports.fd_advise as (...args: unknown[]) => void)(...args); }
export function fd_allocate(...args: unknown[]) { return (context.exports.fd_allocate as (...args: unknown[]) => void)(...args); }
export function fd_close(...args: unknown[]) { return (context.exports.fd_close as (...args: unknown[]) => void)(...args); }
export function fd_datasync(...args: unknown[]) { return (context.exports.fd_datasync as (...args: unknown[]) => void)(...args); }
export function fd_fdstat_get(...args: unknown[]) { return (context.exports.fd_fdstat_get as (...args: unknown[]) => void)(...args); }
export function fd_fdstat_set_flags(...args: unknown[]) { return (context.exports.fd_fdstat_set_flags as (...args: unknown[]) => void)(...args); }
export function fd_fdstat_set_rights(...args: unknown[]) { return (context.exports.fd_fdstat_set_rights as (...args: unknown[]) => void)(...args); }
export function fd_filestat_get(...args: unknown[]) { return (context.exports.fd_filestat_get as (...args: unknown[]) => void)(...args); }
export function fd_filestat_set_size(...args: unknown[]) { return (context.exports.fd_filestat_set_size as (...args: unknown[]) => void)(...args); }
export function fd_filestat_set_times(...args: unknown[]) { return (context.exports.fd_filestat_set_times as (...args: unknown[]) => void)(...args); }
export function fd_pread(...args: unknown[]) { return (context.exports.fd_pread as (...args: unknown[]) => void)(...args); }
export function fd_prestat_get(...args: unknown[]) { return (context.exports.fd_prestat_get as (...args: unknown[]) => void)(...args); }
export function fd_prestat_dir_name(...args: unknown[]) { return (context.exports.fd_prestat_dir_name as (...args: unknown[]) => void)(...args); }
export function fd_pwrite(...args: unknown[]) { return (context.exports.fd_pwrite as (...args: unknown[]) => void)(...args); }
export function fd_read(...args: unknown[]) { return (context.exports.fd_read as (...args: unknown[]) => void)(...args); }
export function fd_readdir(...args: unknown[]) { return (context.exports.fd_readdir as (...args: unknown[]) => void)(...args); }
export function fd_renumber(...args: unknown[]) { return (context.exports.fd_renumber as (...args: unknown[]) => void)(...args); }
export function fd_seek(...args: unknown[]) { return (context.exports.fd_seek as (...args: unknown[]) => void)(...args); }
export function fd_sync(...args: unknown[]) { return (context.exports.fd_sync as (...args: unknown[]) => void)(...args); }
export function fd_tell(...args: unknown[]) { return (context.exports.fd_tell as (...args: unknown[]) => void)(...args); }
export function fd_write(...args: unknown[]) { return (context.exports.fd_write as (...args: unknown[]) => void)(...args); }
export function path_create_directory(...args: unknown[]) { return (context.exports.path_create_directory as (...args: unknown[]) => void)(...args); }
export function path_filestat_get(...args: unknown[]) { return (context.exports.path_filestat_get as (...args: unknown[]) => void)(...args); }
export function path_filestat_set_times(...args: unknown[]) { return (context.exports.path_filestat_set_times as (...args: unknown[]) => void)(...args); }
export function path_link(...args: unknown[]) { return (context.exports.path_link as (...args: unknown[]) => void)(...args); }
export function path_open(...args: unknown[]) { return (context.exports.path_open as (...args: unknown[]) => void)(...args); }
export function path_readlink(...args: unknown[]) { return (context.exports.path_readlink as (...args: unknown[]) => void)(...args); }
export function path_remove_directory(...args: unknown[]) { return (context.exports.path_remove_directory as (...args: unknown[]) => void)(...args); }
export function path_rename(...args: unknown[]) { return (context.exports.path_rename as (...args: unknown[]) => void)(...args); }
export function path_symlink(...args: unknown[]) { return (context.exports.path_symlink as (...args: unknown[]) => void)(...args); }
export function path_unlink_file(...args: unknown[]) { return (context.exports.path_unlink_file as (...args: unknown[]) => void)(...args); }
export function poll_oneoff(...args: unknown[]) { return (context.exports.poll_oneoff as (...args: unknown[]) => void)(...args); }
export function proc_exit(...args: unknown[]) { return (context.exports.proc_exit as (...args: unknown[]) => void)(...args); }
export function proc_raise(...args: unknown[]) { return (context.exports.proc_raise as (...args: unknown[]) => void)(...args); }
export function sched_yield(...args: unknown[]) { return (context.exports.sched_yield as (...args: unknown[]) => void)(...args); }
export function random_get(...args: unknown[]) { return (context.exports.random_get as (...args: unknown[]) => void)(...args); }
export function sock_recv(...args: unknown[]) { return (context.exports.sock_recv as (...args: unknown[]) => void)(...args); }
export function sock_send(...args: unknown[]) { return (context.exports.sock_send as (...args: unknown[]) => void)(...args); }
export function sock_shutdow(...args: unknown[]) { return (context.exports.sock_shutdow as (...args: unknown[]) => void)(...args); }

