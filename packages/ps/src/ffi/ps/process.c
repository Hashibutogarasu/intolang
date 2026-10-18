#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

typedef struct {
    int32_t pid;
} ps_signal;

/* The first field mirrors `Process.pid`, so Into code reads it in place. */
typedef struct {
    int32_t pid;
    int32_t owned;
    int32_t current;
    int32_t reaped;
    int32_t status;
    ps_signal signal;
} ps_process;

/* The fields mirror `Usage`. */
typedef struct {
    int64_t user_micros;
    int64_t system_micros;
    int64_t max_resident_kilobytes;
} ps_usage;

#define PS_ARGUMENTS 30

static int current_taken = 0;

static ps_process *create(int32_t pid, int32_t owned, int32_t current) {
    ps_process *process = (ps_process *)calloc(1, sizeof(ps_process));
    if (!process) {
        return NULL;
    }
    process->pid = pid;
    process->owned = owned;
    process->current = current;
    process->signal.pid = pid;
    return process;
}

ps_process *ps_process_current(void) {
    if (current_taken) {
        return NULL;
    }
    ps_process *process = create((int32_t)getpid(), 1, 1);
    if (process) {
        current_taken = 1;
    }
    return process;
}

ps_process *ps_process_attach(int32_t pid) {
    if (pid <= 0) {
        return NULL;
    }
    return create(pid, 0, 0);
}

ps_process *ps_process_spawn(const char *path, const char *args) {
    if (!path) {
        return NULL;
    }
    char *words = args ? strdup(args) : NULL;
    char *argv[PS_ARGUMENTS + 2];
    int count = 0;
    argv[count++] = (char *)path;
    if (words) {
        for (char *word = strtok(words, " "); word && count <= PS_ARGUMENTS;
             word = strtok(NULL, " ")) {
            argv[count++] = word;
        }
    }
    argv[count] = NULL;
    fflush(NULL);
    pid_t pid = fork();
    if (pid == 0) {
        execv(path, argv);
        _exit(127);
    }
    free(words);
    if (pid < 0) {
        return NULL;
    }
    return create((int32_t)pid, 1, 0);
}

ps_signal *ps_process_signal(ps_process *process) {
    return process ? &process->signal : NULL;
}

ps_usage *ps_process_usage(ps_process *process) {
    ps_usage *usage = (ps_usage *)calloc(1, sizeof(ps_usage));
    if (!usage || !process) {
        return usage;
    }
    struct rusage measured;
    int who = process->current ? RUSAGE_SELF : RUSAGE_CHILDREN;
    if (getrusage(who, &measured) == 0) {
        usage->user_micros =
            (int64_t)measured.ru_utime.tv_sec * 1000000 + measured.ru_utime.tv_usec;
        usage->system_micros =
            (int64_t)measured.ru_stime.tv_sec * 1000000 + measured.ru_stime.tv_usec;
        usage->max_resident_kilobytes = (int64_t)measured.ru_maxrss;
    }
    return usage;
}

void ps_usage_release(ps_usage *usage) {
    free(usage);
}

int32_t ps_process_kill(ps_process *process) {
    if (!process) {
        return -1;
    }
    return (int32_t)kill((pid_t)process->pid, SIGKILL);
}

int32_t ps_process_reap(ps_process *process) {
    if (!process || process->current) {
        return -1;
    }
    if (process->reaped) {
        return process->status;
    }
    int status = 0;
    pid_t result;
    do {
        result = waitpid((pid_t)process->pid, &status, 0);
    } while (result < 0 && errno == EINTR);
    if (result < 0) {
        return -1;
    }
    process->reaped = 1;
    if (WIFEXITED(status)) {
        process->status = WEXITSTATUS(status);
    } else if (WIFSIGNALED(status)) {
        process->status = -WTERMSIG(status);
    } else {
        process->status = -1;
    }
    return process->status;
}

/* Release the handle only: neither the current process nor a child is ever
 * terminated, and a child that was not waited for keeps running. */
void ps_process_release(ps_process *process) {
    if (!process) {
        return;
    }
    if (process->current) {
        current_taken = 0;
    } else if (process->owned && !process->reaped) {
        signal(SIGCHLD, SIG_IGN);
    }
    free(process);
}

void ps_process_detach(ps_process *process) {
    ps_process_release(process);
}

int32_t ps_signal_send(ps_signal *target, int32_t number) {
    if (!target) {
        return -1;
    }
    return (int32_t)kill((pid_t)target->pid, number);
}

#define PS_QUEUE 64

static volatile sig_atomic_t queue[PS_QUEUE];
static volatile sig_atomic_t head = 0;
static volatile sig_atomic_t tail = 0;

static void on_signal(int number) {
    int next = (tail + 1) % PS_QUEUE;
    if (next != head) {
        queue[tail] = number;
        tail = next;
    }
}

void ps_signal_watch(int32_t number) {
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    action.sa_handler = on_signal;
    sigemptyset(&action.sa_mask);
    action.sa_flags = SA_RESTART;
    sigaction(number, &action, NULL);
}

int32_t ps_signal_next(void) {
    if (head == tail) {
        return 0;
    }
    int number = queue[head];
    head = (head + 1) % PS_QUEUE;
    return number;
}

static int requested_code = 0;
static int exit_registered = 0;

static void finish_with_requested_code(void) {
    fflush(NULL);
    _exit(requested_code);
}

/* The program ends with the requested code once `main` returned, which is
 * after every scope destroyed what it owned. */
void ps_exit_request(int32_t code) {
    requested_code = code;
    if (!exit_registered) {
        exit_registered = 1;
        atexit(finish_with_requested_code);
    }
}
