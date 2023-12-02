#!/usr/bin/env zsh

zmodload zsh/zutil

args=(
    h=help -help=help
    -in-repo=in_repo
)

zparseopts -D -E -F $args
if [[ $? -ne 0 ]]; then
    exit 1
fi

if [[ $help ]]; then
    printf "Usage: $0 [options...] <file to convert> ...\n"
    printf "\n"
    printf "Options:\n"
    printf "  -h, --help       show help message and exit\n"
    printf "      --in-repo    Convert to chezmoi_modify_manager being installed in the source directory\n"
    printf "\n"
    printf "This script can be used to automatically convert chezmoi_modify_manager 1.x\n"
    printf "to 2.x files. It assumes the file is on standard format (as created by\n"
    printf "chezmoi_ini_add 1.x). You need to manually check that the conversion is correct!\n"
    printf "\n"
    printf "If --in-repo is not given, it is assumed that the chezmoi_modify_manager will\n"
    printf "be in PATH. If given the path assumed is:\n"
    printf "  {{ .chezmoi.sourceDir }}/.utils/chezmoi_modify_manager-{{ .chezmoi.os }}-{{ .chezmoi.arch }}\n"
    exit 0
fi

# Load the options array from $1
load_options_array() {
    # This is a very stupid trick (redefining exec)
    bash -c "exec() { :; }; source $1; export IFS=\$'\\n'; echo \"\${options[*]}\""
}

error() {
    echo "\e[31m$1\e[0m" >&2
}

# Process shell script wrapper in $1 for chezmoi_modify_manager 1.x, output
# 2.x settings file.
process_file() {
    local in_file="$1"
    if [[ ! -f $in_file ]]; then
        error "$in_file: File doesn't exist"
        return 1
    fi
    if ! grep -Fq ".utils/chezmoi_modify_manager/bin/chezmoi_ini_manager.py" $in_file; then
        error "$in_file: File doesn't look like a 1.x modify template"
        return 1
    fi
    if ! grep -Fq 'src_file="${script/.tmpl/.src.ini}"' $in_file; then
        error "$in_file: File doesn't appear to use standard source path"
        return 1
    fi
    if ! grep -Fq 'options=(' $in_file; then
        error "$in_file: File doesn't appear to use standard options array"
        return 1
    fi
    local -a cmds
    cmds=( "${(f)$(load_options_array "$in_file")}" )

    local -a result_cmds

    while [[ ${#cmds} -gt 0 ]]; do
        case $cmds[1] in
            -ik)
                result_cmds+=("ignore \"$cmds[2]\" \"$cmds[3]\"")
                shift 3 cmds
                ;;
            -ikr)
                result_cmds+=("ignore regex \"$cmds[2]\" \"$cmds[3]\"")
                shift 3 cmds
                ;;
            -tk)
                result_cmds+=("# TODO: Transform arguments need manual adjustment, see documentation")
                result_cmds+=("transform \"$cmds[3]\" \"$cmds[4]\" ${cmds[2]//_/-} \"$cmds[5]\"")
                echo "$in_file: Needs manual adjustment of transform arguments" >&2
                shift 5 cmds
                ;;
            -tkr)
                result_cmds+=("# TODO: Transform arguments need manual adjustment, see documentation")
                result_cmds+=("transform regex \"$cmds[3]\" \"$cmds[4]\" ${cmds[2]//_/-} \"$cmds[5]\"")
                echo "$in_file: Needs manual adjustment of transform arguments" >&2
                shift 5 cmds
                ;;
            -is)
                result_cmds+=("ignore section \"$cmds[2]\"")
                shift 2 cmds
                ;;
            *)
                error "$in_file: Unknown command $cmds[1]"
                return 1
                ;;
        esac
    done

    if [[ $in_repo ]]; then
        printf '#!{{ .chezmoi.sourceDir }}/.utils/chezmoi_modify_manager-{{ .chezmoi.os }}-{{ .chezmoi.arch }}\n'
    else
        printf '#!/usr/bin/env chezmoi_modify_manager\n'
    fi

    printf '\n'
    printf 'source "{{ .chezmoi.sourceDir }}/{{ .chezmoi.sourceFile | trimSuffix ".tmpl" | replace "modify_" "" }}.src.ini"\n'
    printf '\n'

    for line in $result_cmds; do
        printf "%s\n" $line
    done
}

# Process all files on the command line.
for f in $@; do
    process_file $f > ${f}.tmp
    if [[ $? -eq 0 ]]; then
        mv ${f}.tmp $f
    else
        rm ${f}.tmp
    fi
done
