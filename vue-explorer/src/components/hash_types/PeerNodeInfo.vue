<template>
  <div class="table-container">
      <table class="table table-striped table-hover">
        <thead>
        <tr>
          <th v-for="field in this.peerNodeFields" :key="field.key">
            {{ formatTitle(field.key) }}
          </th>
        </tr>
        </thead>
        <tbody>
        <tr v-for="peerNode in hashDataInitial" :key="peerNode.public_key">
          <td><HashLink :data="peerNode.public_key" /></td>
          <td> {{peerNode.external_address}} </td>
          <td> {{peerNode.port_offset}} </td>
          <td> {{shortenExeChecksum(peerNode.executable_checksum)}} </td>
          <td> {{peerNode.utxo_distance}} </td>
          <td> {{peerNode.alias}} </td>
          <td> {{peerNode.name}} </td>
          <td><HashLink :data="peerNode.peer_id" /></td>
          <td> {{peerNode.network_environment}} </td>
        </tr>
        </tbody>
      </table>
  </div>
</template>

<script>
import HashLink from "@/components/util/HashLink.vue";
import {toTitleCase} from "@/utils";
export default {
  name: "PeerNodeInfo",
  components: {
    HashLink
  },
  props: {
    hashDataInitial: Array,
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
      peerNodeFields: [
        {key: 'public_key'},
        {key: 'external_address'},
        {key: 'port_offset'},
        {key: 'executable_checksum'},
        {key: 'utxo_distance'},
        {key: 'alias'},
        {key: 'name'},
        {key: 'peer_id'},
        {key: 'network_environment'},
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