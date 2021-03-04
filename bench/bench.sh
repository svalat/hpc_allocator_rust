#!/bin/bash

###########################################################
# configure shell
set -e
set -u

###########################################################
# variables
BENCH_DIR="results/results-$(date +%Y-%m-%d)"
ITERATIONS=4000000
PROGRSS_BAR=
MEMSET=-m
PARALLEL=true
PARALLEL_J=-j1
FUNCS="MALLOC FREE MEMSET FULLOPS"

###########################################################
function run_bench()
{
	local alloc="$1"
	local file="$2"
	shift 2
	
	if [[ -e ${file} ]]; then
		return
	fi

	if [[ ${PARALLEL} == 'true' ]]; then
		if [[ "$alloc" == 'default' ]]; then
			echo "./bench $@ > '$file'" >> ./jobs.lst
		else
			echo "LD_PRELOAD='${alloc}' ./bench $@ > '$file'" >> ./jobs.lst
		fi
	else
		if [[ "$alloc" == 'default' ]]; then
			./bench "$@" > "$file"
		else
			LD_PRELOAD="${alloc}" ./bench "$@" > "$file"
		fi
	fi
}

###########################################################
function run_bench_fixed_size()
{
	local alloc="$1"
	local alloc_name="$(basename ${alloc} | cut -f 1 -d '.' | sed -e 's/^lib//g')"
	for size in 4 8 16 32 64 128 256 512 1024
	do
		for mode in linear full rand
		do
			local file="${BENCH_DIR}/result-fixed-size-${mode}-${size}-${alloc_name}.dat" 
			echo "Runing ${file}..."
			run_bench "${alloc}" "${file}" -r "${mode}" -b 'fixed' -s ${size} -i ${ITERATIONS} ${PROGRSS_BAR} ${MEMSET}
		done
	done
}

###########################################################
function run_bench_rand_size()
{
	local alloc="$1"
	local alloc_name="$(basename ${alloc} | cut -f 1 -d '.' | sed -e 's/^lib//g')"
	for mode in linear full rand
	do
		local file="${BENCH_DIR}/result-rand-size-${mode}--${alloc_name}.dat" 
		echo "Runing ${file}..."
		run_bench "${alloc}" "${file}" -r "${mode}" -b 'rand' -s 8,8,1024 -i ${ITERATIONS} ${PROGRSS_BAR} ${MEMSET}
	done
}

###########################################################
function run_bench_trace_size()
{
	local alloc="$1"
	local alloc_name="$(basename ${alloc} | cut -f 1 -d '.' | sed -e 's/^lib//g')"
	for app in kdevelop
	do
		local file="${BENCH_DIR}/result-trace-size-${app}--${alloc_name}.dat" 
		echo "Runing ${file}..."
		run_bench "${alloc}" "${file}" -t "trace-${app}.raw" -b 'trace' -i ${ITERATIONS} ${PROGRSS_BAR} ${MEMSET}
	done
}

###########################################################
function gen_gnuplot_lines_commands()
{
	local func=$1
	local mode=$2
	local size_mode=$3
	echo "set title '${size_mode^} size, Reuse ${mode}, ${func}'"
	echo "plot \\"
	for alloc in ./allocs/*
	do
		local alloc_name="$(basename ${alloc} | cut -f 1 -d '.' | sed -e 's/^lib//g')"
		local filename="${BENCH_DIR}/result-${size_mode}-size-${mode}-*-${alloc_name}.dat"
		alloc_name=$(echo ${alloc_name} | sed -e 's/_/\\_/g')
		grep ${func} ${filename} > /dev/null && echo "	'<cat ${filename} | grep ${func} | sort -n -k 2' u 2:5 w l title '${alloc_name}' ,\\"
	done
	local alloc_name="default"
	echo "	'<cat ${BENCH_DIR}/result-${size_mode}-size-${mode}-*-${alloc_name}.dat | grep ${func} | sort -n -k 2' u 2:5 w l title '${alloc_name}'"
}

###########################################################
function gen_file_name()
{
	local size_mode=$1
	local mode=$2
	local alloc_name=$3
	shift 3

	if [[ ${size_mode} == 'trace' ]]; then
		echo "${BENCH_DIR}/result-${size_mode}-size-${mode}--${alloc_name}.dat"
	else
		for size in $@
		do
			echo "${BENCH_DIR}/result-${size_mode}-size-${mode}-${size}-${alloc_name}.dat"
		done | xargs echo
	fi
}

###########################################################
function gen_gnuplot_bar_commands()
{
	local func=$1
	local mode=$2
	local size_mode=$3
	local sizes="8 64 256"
	local ticks=":xtic(2)"
	echo "set title '${size_mode^} size, Reuse ${mode}, ${func}'"
	echo "plot \\"
	for alloc in ./allocs/*
	#for alloc in ./allocs/*mpc* ./allocs/*hpc*
	do
		local alloc_name="$(basename ${alloc} | cut -f 1 -d '.' | sed -e 's/^lib//g')"
		local filename=$(gen_file_name ${size_mode} ${mode} ${alloc_name} ${sizes})
		alloc_name=$(echo ${alloc_name} | sed -e 's/_/\\_/g')
		grep ${func} ${filename} > /dev/null && echo "	'<cat ${filename} | grep ${func} | sort -n -k 2' u 5:4:6${ticks} title '${alloc_name}' ,\\" && ticks=""
	done
	local alloc_name="default"
	echo "	'<cat $(gen_file_name ${size_mode} ${mode} ${alloc_name} ${sizes}) | grep ${func} | sort -n -k 2' u 5:4:6 title '${alloc_name}'"
}

###########################################################
function gen_gnuplot_bar_2_commands()
{
	local func=$1
	local mode=$2
	local size_mode=$3
	local sizes="8 64 256 1024"
	local ticks=":xticlabels(8)"
	echo "set title '${size_mode^} size, Reuse ${mode}, ${func}'"
	echo "plot \\"
	for size in ${sizes}
	do
		local fname="/tmp/out-${size}-${size_mode}-${mode}-${func}.dat"
		local fsize=${size}
		if [[ ${size_mode} == 'trace' ]]; then
			fsize=''
		fi
		for alloc in ./allocs/*
		#for alloc in ./allocs/*mpc* ./allocs/*hpc*
		do
			local alloc_name="$(basename ${alloc} | cut -f 1 -d '.' | sed -e 's/^lib//g')"
			local alloc_name_escaped="$(echo ${alloc_name} | sed -e 's/_/-/g')"
			local filter=$(printf "${func}\t${size}\t")
			local fname="${BENCH_DIR}/result-${size_mode}-size-${mode}-${fsize}-${alloc_name}.dat"
			if [[ -e ${fname} ]]; then
				grep "${filter}" ${fname} > /dev/null && printf "$(cat ${fname} | grep "${filter}")\t${alloc_name_escaped}\n"
			fi
		done > ${fname}
	done

	local sep=''
	for size in ${sizes}
	do
		local fname="/tmp/out-${size}-${size_mode}-${mode}-${func}.dat"
		printf "${sep}	'${fname}' u 5:4:6${ticks} title '${size}'"
		ticks=""
		sep=' ,\\\n'
	done
	echo
}

###########################################################
function run_all()
{
	echo > ./jobs.lst
	for alloc in default ./allocs/*
	do
		#run_bench_fixed_size "${alloc}"
		#run_bench_rand_size "${alloc}"
		run_bench_trace_size "${alloc}"
	done
	if [[ ${PARALLEL} == 'true' ]]; then
		parallel ${PARALLEL_J} --bar < ./jobs.lst
	fi
}

###########################################################
function gen_all_gnuplot_lines()
{
	echo "set term pdf"
	echo "set output '${BENCH_DIR}_lines.pdf'"
	echo "set logscale x 2"
	echo "set grid"
	echo "set yrange [0:]"
	echo "set key outside right"
	echo "set xlabel 'size (bytes)'"
	echo "set ylabel 'Average call time (cycles)'"
	for size_mode in fixed rand
	do
		for mode in linear full rand
		do
			for func in ${FUNCS}
			do
				gen_gnuplot_lines_commands ${func} ${mode} ${size_mode}
			done
		done
	done
	echo "set xrange [0:2048]"
	echo "set yrange [0:1000]"
	for app in kdevelop
	do
		for func in ${FUNCS}
		do
			gen_gnuplot_lines_commands ${func} ${app} 'trace'
		done
	done
}

###########################################################
function gen_all_gnuplot_bars()
{
	echo "set term pdf"
	echo "set output '${BENCH_DIR}_bars.pdf'"
	echo "set style data histogram"
	echo "set style histogram cluster gap 1 errorbars"
	echo "set style fill solid border rgb 'black"
	echo "set auto x"
	echo "set yrange [0:]"
	echo "set grid"
	echo "set key outside right"
	echo "set xlabel 'size (bytes)'"
	echo "set ylabel 'Average call time (cycles)'"
	for size_mode in fixed
	do
		for mode in linear full rand
		do
			for func in ${FUNCS}
			do
				gen_gnuplot_bar_commands ${func} ${mode} ${size_mode}
			done
		done
	done
	for app in kdevelop
	do
		for func in ${FUNCS}
		do
			gen_gnuplot_bar_commands ${func} ${app} 'trace'
		done
	done
}

###########################################################
function gen_all_gnuplot_bars_2()
{
	echo "set term pdf"
	echo "set output '${BENCH_DIR}_bars_2.pdf'"
	echo "set style data histogram"
	echo "set style histogram cluster gap 1 errorbars"
	echo "set style fill solid border rgb 'black"
	echo "set auto x"
	echo "set yrange [0:]"
	echo "set grid"
	echo "set key outside right"
	echo "set ylabel 'Average call time (cycles)'"
	echo "set xtics rotate by 45 right"
	for size_mode in fixed
	do
		for mode in linear full rand
		do
			for func in ${FUNCS}
			do
				gen_gnuplot_bar_2_commands ${func} ${mode} ${size_mode}
			done
		done
	done
	for app in kdevelop
	do
		for func in ${FUNCS}
		do
			gen_gnuplot_bar_2_commands ${func} ${app} 'trace'
		done
	done
}

###########################################################
if [[ $# == 0 ]]; then
	echo "$0 run"
	echo "$0 plot"
	exit 1
fi

if [[ $1 == 'run' ]]; then
	mkdir -p ${BENCH_DIR}
	run_all
fi
if [[ $1 == 'run' || $1 == 'plot' ]]; then
	if [[ ! -d ${BENCH_DIR} ]]; then
		echo "Not data to plot !"
		exit 1
	fi
	gen_all_gnuplot_lines | gnuplot
	gen_all_gnuplot_bars | gnuplot
	gen_all_gnuplot_bars_2 | gnuplot
fi
