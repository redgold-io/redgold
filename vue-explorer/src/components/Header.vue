<template>
  <div class="container-fluid">
    <nav class="navbar custom-navbar">
      <div class="navbar-container">
        <a class="navbar-brand brand" href="/">
          <img :src="require('@/assets/logo.png')" alt="Logo" class="logo">
          Redgold Explorer
        </a>
        <a class="navbar-brand brand" href="https://redgold.io">
          Website
        </a>
        <a class="navbar-brand brand" href="https://dev.docs.redgold.io">
          Docs
        </a>
        <a class="navbar-brand brand" href="https://discord.gg/86fzxJg8ce">
          Discord
        </a>
        <a class="navbar-brand brand" href="/pools">
          Pools
        </a>
        <a class="navbar-brand brand" href="/faucet">
          Faucet
        </a>
        <a class="navbar-brand brand" href="https://grafana.redgold.io/d/pj3zzDu4z/redgold?orgId=1&from=now-30m&to=now">
          Grafana
        </a>
        <div class="navbar-right-links">
          <a class="navbar-brand brand" href="https://dev.explorer.redgold.io">
            Dev
          </a>
          <a class="navbar-brand brand" href="https://staging.explorer.redgold.io">
            Staging
          </a>
          <a class="navbar-brand brand" href="https://test.explorer.redgold.io">
            Test
          </a>
          <a class="navbar-brand brand" href="https://explorer.redgold.io">
            Main
          </a>
        </div>

      </div>

      <!-- Navbar items -->
<!--      <ul class="navbar-nav ml-auto">-->
<!--        <li class="nav-item">-->
<!--          <h6 class="nav-link">Home</h6>-->
<!--        </li>-->
<!--      </ul>-->
    </nav>
    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div class="hash-container">
          <div>Swap Deposit Address: {{ btcSwapAddress ? '' : 'loading...' }} </div>
          <HashLink v-if="btcSwapAddress !== ''" :data="btcSwapAddress" :shorten=false />
          <div>{{ rgdBtcStr }} RDG/BTC</div>
          <div>${{ usdRdgStr }} USD/RDG</div>
          <div>${{ usdBtcStr }} USD/BTC</div>
        </div>



        <!-- Search bar -->
        <div class="jumbotron search-bar py-4">
          <h5 class="text-light">Enter hash search query:</h5>
          <form class="d-flex align-items-center" :action="'/hash/' + searchValue" method="get">
<!--            <label for="inline-form-input-name" class="sr-only">Search</label>-->
            <input v-model="searchValue" id="inline-form-input-name" class="form-control mr-2 search-input" type="text" placeholder="Query hash...">
            <button type="submit" class="btn btn-primary">Submit</button>
          </form>
        </div>
      </div>
    </div>

  </div>
</template>

<script>
import fetchHashInfo from "@/components/mixins/fetchHashInfo";
import HashLink from "@/components/util/HashLink.vue";

export default {
  name: 'HeaderBox',
  components: {
    HashLink,
  },
  data() {
    return {
      searchValue: '',
      btcSwapAddress: '',
      rgdBtc: 100.012312,
      rgdBtcStr: '100.012312',
      usdRdg: 100.012312,
      usdRdgStr: '1.012',
      usdBtc: 30000.3210,
      usdBtcStr: '30000.32'
    };
  },
  mixins: [fetchHashInfo],
  methods: {
    handleSubmit() {
      this.$router.push(`/hash/${this.searchValue}`);
    }
  },
  async created() {
    this.usdBtc = await this.btcUsdPrice();
    // Commit the value to the store
    this.$store.commit('setBtcExchangeRate', this.usdBtc);

    console.log(this.usdBtc);
    this.usdBtcStr = this.usdBtc.toFixed(2);
    let swapInfo = await this.fetchSwapInfo();
    if (swapInfo != null) {
      this.btcSwapAddress = swapInfo.addresses['Bitcoin'];
      if ('Bitcoin' in swapInfo.central_prices) {
        this.usdRdg = swapInfo.central_prices['Bitcoin'].min_ask_estimated
        this.rgdBtc = swapInfo.central_prices['Bitcoin'].min_ask
        this.rgdBtcStr = this.rgdBtc.toFixed(2);
        this.usdRdgStr = this.usdRdg.toFixed(2);
      }
    }
  }
}
</script>

<style scoped>
.custom-navbar {
  background-color: #000;
}

.logo {
  height: 50px;
  width: 50px;
  margin-right: 10px;
}

.custom-navbar .brand {
  color: #fff;
  user-select: none;
  text-decoration: none;
}

.custom-navbar .brand:hover {
  background-color: #000;
  color: #fff;
}

.search-bar {
  background-color: #000;
}

.search-input,
.search-input:focus {
  box-sizing: border-box;
  min-width: 600px;
  max-width: 600px;
  background-color: #191a19;
  color: #fff;
}
.search-input::placeholder {
  color: #ccc;
}


.hash-container {
  display: flex;
  align-items: center;
  gap: 10px; /* Set the gap you want */
}

/* Ensure the navbar container is using flexbox */
.navbar-container {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%; /* Make sure the container spans the full width */
  padding: 0; /* Remove padding to ensure alignment to the edges */
}

/* Align the right links to the far right */
.navbar-right-links {
  margin-left: auto; /* This will push the .navbar-right-links to the right */
  display: flex;
  align-items: center;
  gap: 10px;
}

.navbar-brand {
  margin-right: 7px;
  margin-left: 7px;
}

</style>
