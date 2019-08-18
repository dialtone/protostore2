#![feature(async_await)]

use tokio::net::TcpListener;

use tokio::runtime::current_thread;

use futures::future;

use std::cmp;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use libc;
use log::{debug, info};

use hwloc::{CpuSet, ObjectType, Topology, CPUBIND_THREAD};

use env_logger;

use protostore::{ProtostoreServer, TableOfContents};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cores = hwloc_cores();
    let processing_units = hwloc_processing_units();
    let mut pu_index = 1; // 0 is reserved for main thread
    info!(
        "Found total of {} cores, total of {} processing units",
        cores.len(),
        processing_units.len()
    );

    let short_circuit_reads = false;

    //
    // Read Table of Contents
    //
    let data_dir = Path::new("./db");
    let toc =
        Arc::new(TableOfContents::from_path(data_dir).expect("Could not open table of contents"));
    let max_value_len = toc.max_len();

    //
    // Create threads for handling client comms
    //

    let num_tcp_threads = 5;
    let mut tcp_threads = vec![];

    let (remote_tx, remote_rx) = mpsc::channel();
    for i in 0..num_tcp_threads {
        let pu = cmp::min(pu_index, processing_units.len() - 1);
        pu_index += 1;
        info!("tcp_loop id:{} processing_unit:{}", i, pu);

        let remote_tx = remote_tx.clone();
        let tid = thread::spawn(move || {
            debug!("started thread with id {:?}", unsafe {
                libc::pthread_self()
            });
            bind_thread_to_processing_unit(unsafe { libc::pthread_self() }, pu);
            let mut rt = current_thread::Runtime::new().unwrap();
            remote_tx.send(rt.handle()).unwrap();
            let handle = rt.handle();
            let _ = handle.spawn(future::pending());
            let _ = rt.run();
            debug!("thread ending?");
        });
        tcp_threads.push(tid);
    }

    let tcp_handles: Vec<current_thread::Handle> =
        remote_rx.into_iter().take(num_tcp_threads).collect();
    let tcp_handles_index = AtomicUsize::new(0);

    info!("listening");
    let addr = "0.0.0.0:8080".parse()?;
    let mut listener = TcpListener::bind(&addr).unwrap();

    // Bind the thread that accepts new connections to a dedicated
    // processing unit where we also run other low-intensity tasks
    bind_thread_to_processing_unit(unsafe { libc::pthread_self() }, 0);

    info!("Now accepting connections on {}", addr);
    loop {
        let (socket, _) = listener.accept().await?;
        info!("Got new connection from {}", addr);

        let tcp_idx = tcp_handles_index.fetch_add(1, Ordering::SeqCst) % num_tcp_threads;
        let tcp_handle = &tcp_handles[tcp_idx];

        let toc = toc.clone();
        let _r = tcp_handle.spawn(async move {
            let mut server = ProtostoreServer::new(socket, toc, max_value_len, short_circuit_reads);
            let _ = server.handle_client().await;
        });
    }
}

fn hwloc_processing_units() -> Vec<CpuSet> {
    let topo = Topology::new();
    let cores = topo.objects_with_type(&ObjectType::PU).unwrap();
    cores
        .iter()
        .map(|c| c.cpuset().unwrap())
        .collect::<Vec<CpuSet>>()
}

fn hwloc_cores() -> Vec<CpuSet> {
    let topo = Topology::new();
    let cores = topo.objects_with_type(&ObjectType::Core).unwrap();
    cores
        .iter()
        .map(|c| c.cpuset().unwrap())
        .collect::<Vec<CpuSet>>()
}

fn bind_thread_to_processing_unit(thread: libc::pthread_t, idx: usize) {
    let mut topo = Topology::new();
    let bind_to = match topo.objects_with_type(&ObjectType::PU).unwrap().get(idx) {
        Some(val) => val.cpuset().unwrap(),
        None => panic!("No processing unit found for idx {}", idx),
    };
    topo.set_cpubind_for_thread(thread, bind_to, CPUBIND_THREAD)
        .expect("Could not set cpubind for thread");
}
