<template>
  <div class="table-container">
    <table class="table table-striped table-hover">
      <thead>
      <tr>
        <th v-for="field in this.fields" :key="field.key">
          {{ formatTitle(field.key) }}
        </th>
      </tr>
      </thead>
      <tbody>
      <tr v-for="(d, index) in data" :key="index">
        <td><HashLink :data="d.peer_id" /></td>
        <td> {{d.nodes.length}} </td>
        <td><HashLink :data="d.nodes[0].public_key" /></td>
        <td> {{d.nodes[0].external_address}} </td>
        <td> {{d.nodes[0].port_offset}} </td>
        <td> {{shortenExeChecksum(d.nodes[0].executable_checksum)}} </td>
        <td> {{d.nodes[0].node_name}} </td>
      </tr>
      </tbody>
    </table>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
// import RenderTime from "@/components/RenderTime.vue";
import {toTitleCase} from "@/utils";

export default {
  name: "BriefPeer",
  components: {
    HashLink,
    // RenderTime
  },
  props: {
    data: Object,
  },
  methods: {
    // Now you can use toTitleCase in this component
    formatTitle(key) {
      return toTitleCase(key);
    },
    shortenExeChecksum(h) {
      return h.substring(h.length - 8)
    }
  },
  data() {
    return {
      fields: [
        {key: 'peer_id'},
        {key: 'nodes'},
        {key: 'public_key'},
        {key: 'Host'},
        {key: 'Port'},
        {key: 'Exe Checksum'},
        {key: 'node_name'},
          // TODO: Last updated
          // Last observation, etc.
      ],
    };
  }
}

</script>

<style>
.table .tr .td {
  color: #ffffff;
  background-color: #000000 !important;
}

.table-hover tbody tr:hover {
  color: #FFFFFF !important;
  background-color: #191a19 !important;
}

.table-striped>tbody>tr:nth-child(odd)>td,
.table-striped>tbody>tr:nth-child(odd)>th {
  background-color: #000000 !important;
  color: #ffffff;
}

.table-striped>tbody>tr:nth-child(even)>td,
.table-striped>tbody>tr:nth-child(even)>th {
  background-color: #191a19 !important;
  color: #ffffff;
}

.table-striped>tbody>tr:hover>td,
.table-striped>tbody>tr:hover>th {
  background-color: #291a00 !important; /* Set the desired color on hover */
  color: #ffffff;
}

th {
  background-color: #191a19 !important;
  color: #ffffff !important;
}

</style>