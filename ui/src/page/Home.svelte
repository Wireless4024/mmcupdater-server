<script lang="ts">
	import {
		onDestroy,
		onMount
	}                           from "svelte"
	import {_}                  from "svelte-i18n"
	import {
		Card,
		CardBody,
		CardHeader,
		CardText,
		CardTitle,
		Col,
		ListGroup,
		ListGroupItem,
		Row,
		Table,
		Tooltip
	}                           from "sveltestrap"
	import {get_all_instances}  from "../api/instance"
	import {USER}               from "../api/shared_state"
	import type {SystemInfo}    from "../api/system"
	import {info}               from "../api/system"
	import Container            from "../lib/Container.svelte";
	import ProgressAuto         from "../lib/ProgressAuto.svelte"
	import {percent}            from "../util/math"
	import {memory_unit_from_k} from "../util/memory_unit"

	let sys_info: SystemInfo = {} as any
	let ival = 0

	let instances: string[] = []

	async function load_info() {
		if (!$USER) return
		sys_info = await info()
		instances = await get_all_instances()
	}

	onMount(function () {
		load_info()
		ival = setInterval(load_info, 5000)
	})

	onDestroy(function () {
		ival && clearInterval(ival)
	})
</script>
<Container>
	{#if $USER}
		<Card class="mb-3">
			<CardHeader>
				<CardTitle>{$_("page.instance")}</CardTitle>
			</CardHeader>
			<CardBody>
				<CardText>
					<ListGroup flush>
						{#each instances as instance}
							<ListGroupItem action tag="a" href="#/instance/{instance}">{instance}</ListGroupItem>
						{/each}
					</ListGroup>
				</CardText>
			</CardBody>
		</Card>
		<Card class="mb-3">
			<CardHeader>
				<CardTitle>{$_("page.system")}</CardTitle>
			</CardHeader>
			<CardBody>
				<Table hover>
					<tbody>
					<tr>
						<th scope="row">{$_("sys.hostname")}</th>
						<td>{sys_info.hostname}</td>
					</tr>
					<tr>
						<th scope="row">{$_("sys.os")}</th>
						<td>{sys_info.os}</td>
					</tr>
					<tr>
						<th scope="row">{$_("sys.arch")}</th>
						<td>{sys_info.arch}</td>
					</tr>
					<tr>
						<th scope="row">{$_("sys.cpus")}</th>
						<td>{sys_info.cpus}</td>
					</tr>
					<tr>
						<th scope="row">{$_("sys.cpu_clock")}</th>
						<td>{sys_info.cpu_clock}</td>
					</tr>
					<tr>
						<td colspan="2">
							<Row id="mem">
								<Col xs="6" class="fw-bold">
									{$_("sys.mem")}
								</Col>
								<Col xs="6">
									{memory_unit_from_k(sys_info.mem_used)} / {memory_unit_from_k(sys_info.mem_total)} <br>
								</Col>
								<Col>
									<ProgressAuto
											value={percent(sys_info.mem_used,sys_info.mem_total)*100}/>
								</Col>
							</Row>
						</td>
					</tr>
					<tr>
						<td colspan="2">
							<Row id="mem">
								<Col xs="6" class="fw-bold">
									{$_("sys.swap")}
								</Col>
								<Col xs="6">
									{memory_unit_from_k(sys_info.swap_total - sys_info.swap_free)}
									/ {memory_unit_from_k(sys_info.swap_total)}
								</Col>
								<Col>
									<ProgressAuto
											value={percent(sys_info.swap_total - sys_info.swap_free,sys_info.swap_total)*100}/>
								</Col>
							</Row>
						</td>
					</tr>
					<tr>
						<td colspan="2">
							<Row id="load">
								<Col xs="6" class="fw-bold">
									{$_("sys.load")}
								</Col>
								<Col xs="6">
									{sys_info.load_1?.toFixed(2)} / {sys_info.load_5?.toFixed(2)}
									/ {sys_info.load_15?.toFixed(2)}
								</Col>
								<Col>
									<ProgressAuto
											value={percent(sys_info.load_1,sys_info.cpus)*100}/>
								</Col>
							</Row>
						</td>
					</tr>
					</tbody>
				</Table>
				<Tooltip target="mem" placement="top">{$_("sys.mem_buff")} {memory_unit_from_k(sys_info.mem_buff)} <br>
					{$_("sys.mem_cache")} {memory_unit_from_k(sys_info.mem_cache)} <br>
					{$_("sys.mem_shm")} {memory_unit_from_k(sys_info.mem_shm)} <br>
					{$_("sys.mem_free")} {memory_unit_from_k(sys_info.mem_free)} <br>
					{$_("sys.mem_avail")} {memory_unit_from_k(sys_info.mem_avail)}
				</Tooltip>
				<Tooltip target="load"
				         placement="top">{$_("sys.load_info", {values: {max: ((sys_info.cpus + 10) / 10) + sys_info.cpus}})}</Tooltip>
			</CardBody>
		</Card>
	{/if}
</Container>