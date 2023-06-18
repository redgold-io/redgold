<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div>
          <div class="hash-container">
            <h3 class="detail-group">Address Details</h3>
            <div class="radio-holder" style="display: inline-block; margin-left: 10px;">
              <label class="radio-option"><input type="radio" value="all" v-model="transactionType" /> All</label>
              <label class="radio-option"><input type="radio" value="incoming" v-model="transactionType" />Incoming</label>
              <label class="radio-option"><input type="radio" value="outgoing" v-model="transactionType" />Outgoing</label>
            </div>
          </div>
          <div class="grid-container">

            <div><strong>Address</strong></div>
            <div class="hash-container">
              {{ hashData.address }}
              <div><CopyClipboard :data="hashData.address" /></div>
            </div>

            <div><strong>Balance</strong></div>
            <div>{{ hashData.balance }} RDG</div>

            <div><strong>Total UTXOs</strong></div>
            <div>{{ hashData.total_utxos }}</div>

            <div><strong>Total Transactions</strong></div>
            <div>{{ hashData.total_count }}</div>

            <div><strong>Incoming Transactions</strong></div>
            <div>{{ hashData.incoming_count }}</div>

            <div><strong>Outgoing Transactions</strong></div>
            <div>{{ hashData.outgoing_count }}</div>

          </div>
          <h3 class="detail-group">Transactions</h3>
          <div><BriefTransaction :transactions="filteredTransactions" /></div>
          <nav>
            <ul class="pagination">
              <li class="page-item" :class="{ 'disabled': currentPage === 1 }">
                <a class="page-link" href="#" @click.prevent="goToPage(1)" :aria-disabled="currentPage === 1">First</a>
              </li>

              <li class="page-item" :class="{ 'disabled': currentPage === 1 }">
                <a class="page-link" href="#" @click.prevent="currentPage--" :aria-disabled="currentPage === 1">Previous</a>
              </li>

              <li class="page-item" v-for="page in visiblePages" :key="page" :class="{ 'active': page === currentPage }">
                <a class="page-link" href="#" @click.prevent="goToPage(page)">{{ page }}</a>
              </li>

              <li class="page-item" :class="{ 'disabled': currentPage === pageCount }">
                <a class="page-link" href="#" @click.prevent="currentPage++" :aria-disabled="currentPage === pageCount">Next</a>
              </li>

              <li class="page-item" :class="{ 'disabled': currentPage === pageCount }">
                <a class="page-link" href="#" @click.prevent="goToPage(pageCount)" :aria-disabled="currentPage === pageCount">Last</a>
              </li>

            </ul>
          </nav>

        </div>
      </div>
    </div>
  </div>
</template>


<script>

// import HashLink from "@/components/HashLink.vue";
import CopyClipboard from "@/components/util/CopyClipboard.vue";
// import RenderTime from "@/components/RenderTime.vue";
import BriefTransaction from "@/components/BriefTransaction.vue";
import fetchHashInfo from "@/components/mixins/fetchHashInfo";

export default {
  name: 'TransactionDetail',
  props: ['hashDataInitial'],
  components: {
    BriefTransaction,
    // RenderTime,
    // HashLink,
    CopyClipboard
  },
  data: function() {
    return {
      transactionType: 'all',
      currentPage: 1,
      perPage: 10,
      hashData: this.hashDataInitial
    }
  },
  mixins: [fetchHashInfo],
  computed: {
    filteredTransactions() {
      if (this.transactionType === 'incoming') {
        return this.hashData.incoming_transactions;
      } else if (this.transactionType === 'outgoing') {
        return this.hashData.outgoing_transactions;
      }
      return this.hashData.recent_transactions;
    },
    numTransactions() {
      if (this.transactionType === 'incoming') {
        return this.hashData.incoming_count;
      } else if (this.transactionType === 'outgoing') {
        return this.hashData.outgoing_count;
      }
      return this.hashData.total_count;
    },
    pageCount() {
      return Math.ceil(this.numTransactions / this.perPage);
    },
    visiblePages() {
      let startPage = Math.max(1, this.currentPage - 5);
      let endPage = Math.min(this.pageCount, this.currentPage + 5);

      let pages = [];
      for (let i = startPage; i <= endPage; i++) {
        pages.push(i);
      }
      return pages;
    },
  },
  methods: {
    async goToPage(page) {
      if (page !== this.currentPage) {
        this.currentPage = page;

        // Calculate offset and limit for fetching data
        let offset = (this.currentPage - 1) * this.perPage;
        let limit = this.perPage;

        // Fetch data and update hashData
        await this.fetchData(null, "16481",  offset, limit)
      }
    }
  }

}
</script>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: 1fr 6fr; /* Adjust as needed */
  gap: 10px; /* Adjust as needed */
  padding-top: 5px;
  padding-bottom: 5px;
}

.hash-container {
  display: flex;
  align-items: center;
}
.detail-group {
  padding-top: 15px;
  padding-bottom: 15px;
  padding-left: 10px;
  padding-right: 20px;
  background-color: #191a19 !important;
}
.signature {
  word-break: break-word;
  overflow-wrap: break-word;
}

.radio-option {
  margin-right: 20px;
}

.radio-option input[type="radio"] {
  //display: none;
}

.radio-option span {
  padding: 10px;
  border: 1px solid #ccc;
  display: inline-block;
  margin-right: 5px;
  cursor: pointer;
}

.radio-option input[type="radio"]:checked + span {
  //background-color: #ddd;
}

.radio-holder {
  padding-left: 10px;
  background-color: #191a19 !important;
}

.pagination {
  background-color: #000000; /* slightly lighter grey for the active page */
}

.page-link {
  color: #fff; /* white text */
  background-color: #000000; /* slightly lighter grey for the active page */
}

.page-item.active .page-link {
  background-color: #000000; /* slightly lighter grey for the active page */
  border-color: #666;
}

.page-item.disabled .page-link {
  color: #999; /* lighter grey text for disabled buttons */
}


</style>